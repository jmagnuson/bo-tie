use alloc::{
    boxed::Box,
    vec::Vec,
};
use core::future::Future;
use crate::{ att, l2cap, UUID};

pub mod characteristic;

struct ServiceDefinition;

impl ServiceDefinition {
    /// The permissions of the service definitions is just Read Only
    const PERMISSIONS: &'static [att::AttributePermissions] = &[att::AttributePermissions::Read];

    /// The primary service UUID
    const PRIMARY_SERVICE_TYPE: UUID = UUID::from_u16(0x2800);

    /// The secondary service UUID
    const SECONDARY_SERVICE_TYPE: UUID = UUID::from_u16(0x2801);
}

struct ServiceInclude {
    service_handle: u16,
    end_group_handle: u16,
    short_service_type: Option<u16>,
}

impl att::TransferFormat for ServiceInclude {
    fn from(raw: &[u8]) -> Result<Self, att::TransferFormatError> {
        // The implementation of TransferFormat for UUID will check if the length is good for
        // a 128 bit UUID
        if raw.len() >= 6 {
            Ok( ServiceInclude {
                service_handle: att::TransferFormat::from( &raw[..2] )?,
                end_group_handle: att::TransferFormat::from( &raw[2..4] )?,
                short_service_type: if raw[4..].len() == 2 {
                    // Only 16 Bluetooth UUIDs are included with a Include Definition

                    Some( att::TransferFormat::from( &raw[4..])? )
                } else if raw[4..].len() == 0 {
                    None
                } else {
                    return Err(att::TransferFormatError::from(
                        concat!("Invalid short service type in ", stringify!("ServiceInclude"))))
                },
            })
        } else {
            Err( att::TransferFormatError::bad_min_size(stringify!(ServiceInclude),
                6, raw.len()) )
        }
    }

    fn into(&self) -> Box<[u8]> {
        let mut v = Vec::new();

        v.extend_from_slice( &att::TransferFormat::into(&self.service_handle) );
        v.extend_from_slice( &att::TransferFormat::into(&self.end_group_handle) );

        if let Some(uuid_ref) = &self.short_service_type {
            v.extend_from_slice( &att::TransferFormat::into(uuid_ref) );
        }

        v.into_boxed_slice()
    }
}

impl ServiceInclude {
    const TYPE: UUID = UUID::from_u16(0x2802);

    const PERMISSIONS: &'static [att::AttributePermissions] = &[att::AttributePermissions::Read];
}

pub struct ServiceBuilder<'a>
{
    service_type: UUID,
    /// The list of primary services. This is none if the service builder is constructing a
    /// secondary service.
    is_primary: bool,
    handle: u16,
    server_builder: &'a mut ServerBuilder,
}

impl<'a> ServiceBuilder<'a>
{

    fn new(
        server_builder: &'a mut ServerBuilder,
        service_type: UUID,
        is_primary: bool
    ) -> Self
    {
        let handle = server_builder.attributes.push(
            att::Attribute::new(
                if is_primary {
                    ServiceDefinition::PRIMARY_SERVICE_TYPE
                } else {
                    ServiceDefinition::SECONDARY_SERVICE_TYPE
                },
                ServiceDefinition::PERMISSIONS.into(),
                service_type
            )
        );

        ServiceBuilder { service_type, is_primary, handle, server_builder }
    }

    /// Start including other services
    ///
    /// This converts a `Service Builder` into a `IncludesAdder`. The returned `IncludesAdder`
    /// will allow for the addition of include definitions for other services. Afterwards an
    /// `IncludesAdder` can be further converted into a `CharacteristicAdder`
    pub fn into_includes_adder(self) -> IncludesAdder<'a> {
        IncludesAdder::new(self)
    }

    /// Start adding characteristics
    ///
    /// This converts a `Service Builder` into a `CharacteristicAdder`. Use this function when the
    /// service includes no other services. This will create a
    /// characteristic adder that can be used to add characteristics after the service difinition
    /// attribute. It is not possible to add includes to other services if this function is used.
    ///
    /// If you wish to create a service that includes other services, use the
    /// `[into_includes_adder](#add_service_includes)`
    /// function. That function will return a `IncludesAdder` which can be then converted into
    /// a `CharacteristicAdder` for adding characteristics to the service.
    pub fn into_characteristics_adder(self) -> CharacteristicAdder<'a> {
        let handle = self.handle;
        CharacteristicAdder::new(self, handle)
    }

    /// Create an empty service
    ///
    /// This will create a service with no include definitions or characteristics. This means that
    /// the service will contain no data other then what is in the service definition. As a result
    /// an empty service will only contain its UUID.
    pub fn make_empty(mut self) -> Service {
        // There is only one handle in an empty Service so both the service handle and end group
        // handle are the same
        self.make_service(self.handle)
    }

    fn make_service(&mut self, end_service_handle: u16 ) -> Service {

        let service = Service::new( self.handle, end_service_handle, self.service_type);

        if self.is_primary { self.server_builder.add_primary_service(service)}

        service
    }
}


/// Add Include Definition(s) to the service
///
/// The service that will contain the include definition(s) is the same service that was initially
/// constructing with ServiceBuilder.
///
/// This is created by the
/// `[into_includes_adder](../ServiceBuilder/index.html#into_includes_adder)`
/// function.
pub struct IncludesAdder<'a>
{
    service_builder: ServiceBuilder<'a>,
    end_group_handle: u16
}

impl<'a> IncludesAdder<'a>
{
    fn new( service_builder: ServiceBuilder<'a>)
    -> Self
    {
        let handle = service_builder.handle;

        IncludesAdder {
            service_builder: service_builder,
            end_group_handle: handle,
        }
    }

    /// Add a service to include
    pub fn include_service( mut self, service: &Service ) -> Self {
        use core::convert::TryInto;

        let include = ServiceInclude {
            service_handle: service.service_handle,
            end_group_handle: service.end_group_handle,
            short_service_type: service.service_type.try_into().ok()
        };

        let attribute = att::Attribute::new(
            ServiceInclude::TYPE,
            ServiceInclude::PERMISSIONS.into(),
            include
        );

        self.end_group_handle = self.service_builder.server_builder.attributes.push(attribute);

        self
    }

    /// Convert to a CharacteristicAdder
    pub fn into_characteristics_adder(self) -> CharacteristicAdder<'a> {
        CharacteristicAdder::new(
            self.service_builder,
            self.end_group_handle
        )
    }

    /// Finish the service
    ///
    /// This will create a service that only has the service definition and service includes (if
    /// any). There will be no characteristics added to the service.
    pub fn finish_service(mut self) -> Service {

        self.service_builder.make_service(self.end_group_handle)
    }
}

/// Add characteristics to a service
///
/// The service that will contain the characteristic(s) is the same service that was initially
/// constructing with ServiceBuilder.
///
/// This is created by the
/// `[ServiceBuilder::into_characteristics_adder](../ServiceBuilder/index.html#into_includes_adder)`
/// or
/// `[IncludesAdder::into_characteristics_adder](../ServiceBuilder/index.html#into_includes_adder)`
/// functions.
pub struct CharacteristicAdder<'a>
{
    service_builder: ServiceBuilder<'a>,
    end_group_handle: u16
}

impl<'a> CharacteristicAdder<'a>
{
    fn new(
        service_builder: ServiceBuilder<'a>,
        end_group_handle: u16,
    ) -> Self
    {
        CharacteristicAdder { service_builder, end_group_handle }
    }

    pub fn build_characteristic<V>(
        self,
        properties: Vec<characteristic::Properties>,
        uuid: UUID,
        value: Box<V>,
        value_permissions: Vec<att::AttributePermissions> )
    -> characteristic::CharacteristicBuilder<'a, V>
    where Box<V>: att::TransferFormat + Unpin + 'static,
              V: ?Sized
    {
        characteristic::CharacteristicBuilder::new(
            self,
            properties,
            uuid,
            value,
            value_permissions
        )
    }

    /// Finish the service
    pub fn finish_service(mut self) -> Service {
        self.service_builder.make_service( self.end_group_handle )
    }
}

#[derive(Clone,Copy,PartialEq,PartialOrd,Eq,Ord,Debug)]
pub struct Service {
    /// The handle of the Service declaration attribute
    service_handle: u16,
    /// The handle of the last attribute in the service
    end_group_handle: u16,
    /// The UUID (also known as the attribute type) of the service. This is also the attribute
    /// value in the service definition.
    service_type: UUID,
}

impl Service {

    fn new( service_handle: u16, end_group_handle: u16, service_type: UUID ) -> Self
    {
        Service { service_handle, end_group_handle, service_type }
    }
}

pub struct GapServiceBuilder {
    server_builder: ServerBuilder
}

impl GapServiceBuilder {
    /// Service UUID
    const GAP_SERVICE_TYPE: UUID = UUID::from_u16(0x1800);

    /// Default Appearance
    pub const UNKNOWN_APPERANCE: u16 = 0;

    /// Make a new `GapServiceBuilder`
    ///
    /// The `device_name` is a readable string for the client. The appperance is an assigned number
    /// to indiciate to the client the external appearance of the device. Both these fields are
    /// optional with `device_name` defaulting to an empty string and 'unknown apperance'
    pub fn new<'a,D,A>(device_name: D, apperance: A) -> Self
    where D: Into<Option<&'a str>>,
          A: Into<Option<u16>>
    {
        use characteristic::Properties;
        use att::AttributePermissions;

        let device_name_props = [Properties::Read].to_vec();
        let apperance_props   = [Properties::Read].to_vec();

        let device_name_type = UUID::from_u16(0x2a00);
        let apperance_type   = UUID::from_u16(0x2a01);

        let device_name_val: Box<str> = if let Some(name) = device_name.into() {
            name.into()
        } else {
            "".into()
        };

        let apperance_val = if let Some(appr) = apperance.into() {
            Box::new(appr)
        } else {
            Box::new( Self::UNKNOWN_APPERANCE)
        };

        let device_name_att_perms = [AttributePermissions::Read].to_vec();
        let apperance_att_perms = [AttributePermissions::Read].to_vec();

        let mut server_builder = ServerBuilder::new_empty();

        server_builder.new_service_constructor(Self::GAP_SERVICE_TYPE, true)
        .into_characteristics_adder()
        .build_characteristic(device_name_props, device_name_type, device_name_val, device_name_att_perms)
        .finish_characteristic()
        .build_characteristic(apperance_props, apperance_type, apperance_val, apperance_att_perms)
        .finish_characteristic()
        .finish_service();

        GapServiceBuilder { server_builder }
    }
}

/// Constructor of a GATT server
///
/// This will construct a GATT server for use with BR/EDR/LE bluetooth operation.
pub struct ServerBuilder
{
    primary_services: Vec<Service>,
    attributes: att::server::ServerAttributes,
}

impl ServerBuilder
{

    /// Construct an empty `ServerBuilder`
    ///
    /// This creates a `ServerBuilder` without the specification required GAP service.
    pub fn new_empty() -> Self {
        Self {
            primary_services: Vec::new(),
            attributes: att::server::ServerAttributes::new(),
        }
    }

    /// Construct a new `ServicesBuiler`
    ///
    /// This will make a `ServiceBuilder` with the basic requirements for a GATT server. This
    /// server will only contain a *GAP* service with the characteristics *Device Name* and
    /// *Appearance*, but both of these characteristics contain no information.
    pub fn new() -> Self
    {
        GapServiceBuilder::new("", GapServiceBuilder::UNKNOWN_APPERANCE).server_builder
    }

    /// Construct a new `ServiceBuilder` with the provided GAP service builder
    ///
    /// The provided GAP service builder will be used to construct the required GAP service for the
    /// GATT server.
    pub fn new_with_gap(gap: GapServiceBuilder) -> Self {
        gap.server_builder
    }

    /// Create a service constructor
    pub fn new_service_constructor<'a>(&'a mut self, service_type: UUID, is_primary: bool)
    -> ServiceBuilder<'a>
    {
        ServiceBuilder::new(self, service_type, is_primary)
    }

    /// Make an server
    ///
    /// Construct an server from the server builder.
    pub fn make_server<C,Mtu>(self, connection_channel: C, server_mtu: Mtu)
    -> Server<C>
    where C: l2cap::ConnectionChannel,
          Mtu: Into<Option<u16>>
    {
        Server {
            primary_services: self.primary_services,
            server: att::server::Server::new(connection_channel, server_mtu.into(), Some(self.attributes))
        }
    }

    fn add_primary_service(&mut self, service: Service ) {
        self.primary_services.push(service)
    }
}

pub struct Server<C>
where C: l2cap::ConnectionChannel
{
    primary_services: Vec<Service>,
    server: att::server::Server<C>
}

impl<C> Server<C> where C: l2cap::ConnectionChannel
{
    pub fn receiver<'a>(&'a mut self)
    -> impl Future<Output = Result<GattRequestProcessor<'a, C>, att::Error>> + 'a
    {
        GattReceiver {
            primary_services: &self.primary_services,
            att_receiver: self.server.receiver()
        }
    }
}

impl<C> AsRef<att::server::Server<C>> for Server<C> where C: l2cap::ConnectionChannel {
    fn as_ref(&self) -> &att::server::Server<C> {
        &self.server
    }
}

impl <C> AsMut<att::server::Server<C>> for Server<C> where C: l2cap::ConnectionChannel {
    fn as_mut(&mut self) -> &mut att::server::Server<C> {
        &mut self.server
    }
}

struct GattReceiver<'a, C, R>
where C: l2cap::ConnectionChannel + 'a,
      R: Future<Output = Result<att::server::RequestProcessor<'a, C>, att::Error>> + Unpin + 'a
{
    primary_services: &'a [Service],
    att_receiver: R
}

impl<'a,C,R> Future for GattReceiver<'a, C, R>
where C: l2cap::ConnectionChannel,
      R: Future<Output = Result<att::server::RequestProcessor<'a, C>, att::Error>> + Unpin + 'a
{
    type Output = Result<GattRequestProcessor<'a, C>, att::Error>;

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
        use core::pin::Pin;
        use core::task;

        let this = self.get_mut();

        match Pin::new(&mut this.att_receiver).poll(cx) {
            task::Poll::Pending => task::Poll::Pending,

            task::Poll::Ready(Ok(rp)) =>
                task::Poll::Ready( Ok(
                    GattRequestProcessor {
                        primary_services: this.primary_services,
                        att_recq_proc: rp
                    }
                )),

            task::Poll::Ready(Err(e)) => task::Poll::Ready(Err(e)),
        }
    }
}

/// Gatt client requests processor
///
/// This structure is created by polling to completion the future returned by
/// [`Server::receiver`](Server::receiver).
/// This is used to process client requests and send the coresponding response to the client.
/// To process the request, the function
/// [`process_request`](GattRequestProcessor::process_request)
/// must be called.
pub struct GattRequestProcessor<'a,C>
where C: l2cap::ConnectionChannel
{
    primary_services: &'a [Service],
    att_recq_proc: att::server::RequestProcessor<'a, C>
}

impl<'a, C> GattRequestProcessor<'a, C>
where C: l2cap::ConnectionChannel
{
    pub fn process_request(&mut self) -> Result<(), att::Error> {

        match self.att_recq_proc.get_request_type() {
            Some( att::client::ClientPduName::ReadByGroupTypeRequest ) => {
                log::info!("(GATT) processing '{}'", att::client::ClientPduName::ReadByGroupTypeRequest );

                self.process_read_by_group_type_request(
                    att::TransferFormat::from(self.att_recq_proc.get_request_raw_data())?
                )
            },
            _ => self.att_recq_proc.process_request()?
        }

        Ok(())
    }

    fn process_read_by_group_type_request(&self, pdu: att::pdu::Pdu<att::pdu::TypeRequest>)
    {
        let handle_range = &pdu.get_parameters().handle_range;

        let err_rsp = | pdu_err | {

            let handle = handle_range.starting_handle;
            let opcode = pdu.get_opcode().into_raw();

            self.att_recq_proc.as_ref().send_error(handle, opcode, pdu_err);

            None
        };

        if ! handle_range.is_valid() {

            err_rsp( att::pdu::Error::InvalidHandle );

        } else if pdu.get_parameters().attr_type == ServiceDefinition::PRIMARY_SERVICE_TYPE {

            use core::convert::TryInto;

            const REQUIRED_PERMS: &[att::AttributePermissions] = &[
                att::AttributePermissions::Read
            ];

            const RESTRICTED_PERMS: &[att::AttributePermissions] = &[
                att::AttributePermissions::Encryption(att::AttributeRestriction::Read, att::EncryptionKeySize::Bits128),
                att::AttributePermissions::Encryption(att::AttributeRestriction::Read, att::EncryptionKeySize::Bits192),
                att::AttributePermissions::Encryption(att::AttributeRestriction::Read, att::EncryptionKeySize::Bits256),
                att::AttributePermissions::Authentication(att::AttributeRestriction::Read),
                att::AttributePermissions::Authorization(att::AttributeRestriction::Write),
            ];

            let permissions_error = | service: &Service | -> Option<att::pdu::Error> {
                self.att_recq_proc
                .as_ref()
                .check_permission(service.service_handle, REQUIRED_PERMS, RESTRICTED_PERMS)
                .err()
            };

            self.primary_services.iter()
            .filter(|s| s.service_handle >= handle_range.starting_handle)
            .next()
            .or_else( || err_rsp( att::pdu::Error::AttributeNotFound) )
            .and_then( |first_service| -> Option<()> {

                let predicate_short_uuid = |service: &&Service| {
                    service.service_handle <= handle_range.ending_handle &&
                    TryInto::<u16>::try_into(service.service_type).is_ok() &&
                    permissions_error(service).is_none()
                };

                let predicate_normal_uuid = |service: &&Service| {
                    service.service_handle <= handle_range.ending_handle &&
                    permissions_error(service).is_none()
                };

                let ( size, predicate ): (usize, &dyn Fn(&&Service) -> bool) =
                    if TryInto::<u16>::try_into(first_service.service_type).is_ok() {
                        (2, &predicate_short_uuid)
                    } else {
                        (16, &predicate_normal_uuid)
                    };

                match permissions_error(first_service) {
                    None => {
                        let max_data = core::cmp::min(
                            core::u8::MAX as u16,
                            self.att_recq_proc.as_ref().get_mtu()
                        ) as usize;

                        let data_response = self.primary_services
                            .iter()
                            .take_while(predicate)
                            .map( |service|
                                att::pdu::ReadGroupTypeData::new(
                                    service.service_handle,
                                    service.end_group_handle,
                                    service.service_type,
                                )
                            )
                            .enumerate()
                            .take_while( |(cnt,_)| (cnt * (4 + size)) <= max_data )
                            .fold( Vec::new(), |mut v, (_,rgtd)| { v.push(rgtd); v } );

                        let response_data = att::pdu::ReadByGroupTypeResponse::new(data_response)
                            .expect("data_response is empty");

                        let pdu = att::pdu::read_by_group_type_response(response_data);

                        let tx_data = att::TransferFormat::into( &pdu );

                        let acl_data = l2cap::AclData::new(tx_data.to_vec(), att::L2CAP_CHANNEL_ID );

                        self.att_recq_proc.as_ref().as_ref().send(acl_data);
                    },
                    Some(e) => { err_rsp(e); },
                }

                None
            });

        } else {
            err_rsp( att::pdu::Error::UnsupportedGroupType );
        }
    }
}

impl<'a,C> AsRef<att::server::RequestProcessor<'a, C>> for GattRequestProcessor<'a, C>
where C: l2cap::ConnectionChannel
{
    fn as_ref(&self) -> &att::server::RequestProcessor<'a,C> {
        &self.att_recq_proc
    }
}

impl<'a,C> AsMut<att::server::RequestProcessor<'a, C>> for GattRequestProcessor<'a, C>
where C: l2cap::ConnectionChannel
{
    fn as_mut(&mut self) -> &mut att::server::RequestProcessor<'a,C> {
        &mut self.att_recq_proc
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use alloc::boxed::Box;
    use crate::l2cap::ConnectionChannel;
    use crate::UUID;

    struct DummyConnection;

    impl ConnectionChannel for DummyConnection {
        const DEFAULT_ATT_MTU: u16 = core::u16::MAX;

        fn send(&self, _: &[u8]) {}
        fn receive(&self, _: core::task::Waker) -> Option<Box<[u8]>> { None }
    }

    #[test]
    fn create_gatt_attributes() {

        let mut server_builder = ServerBuilder::new();

        let test_service_1 = server_builder.into_service_constructor( UUID::from_u16(0x1234), false )
            .into_characteristics_adder()
            .build_characteristic(
                vec!(characteristic::Properties::Read),
                UUID::from(0x1234u16),
                Box::new(0usize),
                vec!(att::AttributePermissions::Read)
            )
            .set_extended_properties( vec!(characteristic::ExtendedProperties::ReliableWrite) )
            .set_user_description( characteristic::UserDescription::new(
                "Test 1",
                vec!(att::AttributePermissions::Read) )
            )
            .set_client_configuration( vec!(characteristic::ClientConfiguration::Notification) )
            .set_server_configuration( vec!(characteristic::ServerConfiguration::Broadcast) )
            .finish_characteristic()
            .finish_service();

        let _test_service_2 = server_builder.into_service_constructor( UUID::from_u16(0x3456), true )
            .into_includes_adder()
            .include_service(&test_service_1)
            .finish_service();

        server_builder.make_server(DummyConnection, 0xFFu16);
    }
}
