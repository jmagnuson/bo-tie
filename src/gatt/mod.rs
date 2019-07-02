use alloc::{
    boxed::Box,
    vec::Vec,
};
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
    fn from(raw: &[u8]) -> Result<Self, att::pdu::Error> {
        // The implementation of TransferFormat for UUID will check if the length is good for
        // a 128 bit UUID
        if raw.len() >= (4 + core::mem::size_of::<u16>()) {
            Ok( ServiceInclude {
                service_handle: att::TransferFormat::from( &raw[..2] )?,
                end_group_handle: att::TransferFormat::from( &raw[2..4] )?,
                short_service_type: if raw[4..].len() == 2 {
                    // Only 16 Bluetooth UUIDs are included with a Include Definition

                    Some( att::TransferFormat::from( &raw[4..])? )
                } else if raw[4..].len() == 0 {
                    None
                } else {
                    return Err( att::pdu::Error::InvalidPDU )
                },
            })
        } else {
            Err( att::pdu::Error::InvalidPDU )
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
    is_primary: bool,
    attributes: &'a mut att::server::ServerAttributes,
}

impl<'a> ServiceBuilder<'a>
{

    fn new(
        attributes: &'a mut att::server::ServerAttributes,
        service_type: UUID,
        is_primary: bool
    ) -> Self
    {
        ServiceBuilder { service_type, is_primary, attributes }
    }

    fn push_declaration(&mut self) -> u16 {
        self.attributes.push(
            att::Attribute::new(
                if self.is_primary {
                    ServiceDefinition::PRIMARY_SERVICE_TYPE
                } else {
                    ServiceDefinition::SECONDARY_SERVICE_TYPE
                },
                ServiceDefinition::PERMISSIONS.into(),
                self.service_type
            )
        )
    }

    pub fn set_type(&mut self, uuid: UUID ) -> &mut Self {
        self.service_type = uuid;
        self
    }

    pub fn make_primary(&mut self, primary: bool) -> &mut Self {
        self.is_primary = primary;
        self
    }


    /// Start including other services
    ///
    /// This converts a `Service Builder` into a `IncludesAdder`. The returned `IncludesAdder`
    /// will allow for the addition of include definitions for other services. Afterwards an
    /// `IncludesAdder` can be further converted into a `CharacteristicAdder`
    pub fn into_includes_adder(mut self) -> IncludesAdder<'a> {

        let service_handle = self.push_declaration();

        IncludesAdder::new(service_handle, self.service_type, self.attributes)
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
    pub fn into_characteristics_adder(mut self) -> CharacteristicAdder<'a> {

        let handle = self.push_declaration();

        CharacteristicAdder::new( handle, handle, self.service_type, self.attributes)
    }

    /// Create an empty service
    ///
    /// This will create a service with no include definitions or characteristics. This means that
    /// the service will contain no data other then what is in the service definition. As a result
    /// an empty service will only contain its UUID.
    pub fn make_empty(mut self) -> Service {

        let handle = self.push_declaration();

        // There is only one handle in an empty Service so both the service handle and end group
        // handle are the same
        Service::new( handle, handle, self.service_type )
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
    service_handle: u16,
    service_type: UUID,
    attributes: &'a mut att::server::ServerAttributes,
    end_group_handle: core::cell::Cell<u16>
}

impl<'a> IncludesAdder<'a>
{
    fn new( service_handle: u16, service_type: UUID, attributes: &'a mut att::server::ServerAttributes)
    -> Self
    {
        IncludesAdder {
            service_handle: service_handle,
            service_type: service_type,
            attributes: attributes,
            end_group_handle: service_handle.into(),
        }
    }

    /// Add a service to include
    pub fn include_service( self, service: &Service ) -> Self {
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

        self.end_group_handle.set( self.attributes.push(attribute) );

        self
    }

    /// Convert to a CharacteristicAdder
    pub fn into_characteristics_adder(self) -> CharacteristicAdder<'a> {
        CharacteristicAdder::new(
            self.service_handle,
            self.end_group_handle.into_inner(),
            self.service_type,
            self.attributes
        )
    }

    /// Finish the service
    ///
    /// This will create a service that only has the service definition and service includes (if
    /// any). There will be no characteristics added to the service.
    pub fn finish_service(self) -> Service {
        Service::new( self.service_handle, self.end_group_handle.into_inner(), self.service_type )
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
    service_handle: u16,
    service_type: UUID,
    attributes: &'a mut att::server::ServerAttributes,
    end_group_handle: core::cell::Cell<u16>
}

impl<'a> CharacteristicAdder<'a>
{
    fn new(
        service_handle: u16,
        end_group_handle: u16,
        service_type: UUID,
        attributes: &'a mut att::server::ServerAttributes
    ) -> Self
    {
        CharacteristicAdder {
            service_handle: service_handle,
            service_type: service_type,
            attributes: attributes,
            end_group_handle: end_group_handle.into(),
        }
    }

    pub fn build_characteristic<V>(
        self,
        properties: Vec<characteristic::Properties>,
        uuid: UUID,
        value: Box<V>,
        value_permissions: Vec<att::AttributePermissions> )
    -> characteristic::CharacteristicBuilder<'a, V>
    where V: att::TransferFormat + Sized + Unpin + 'static
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
    pub fn finish_service(self) -> Service {
        Service::new( self.service_handle, self.end_group_handle.into_inner(), self.service_type )
    }
}

pub struct Service {
    /// The handle of the Service declaration attribute
    service_handle: u16,
    /// The handle of the last attribute in the service
    end_group_handle: u16,
    /// The UUID (also known as the attribute type) of the service
    service_type: UUID,
}

impl Service {

    fn new( service_handle: u16, end_group_handle: u16, service_type: UUID ) -> Self {
        Service { service_handle, end_group_handle, service_type }
    }
}

pub struct ServerBuilder
{
    attributes: att::server::ServerAttributes
}

impl ServerBuilder
{

    /// Construct a new `ServicesBuiler`
    pub fn new() -> Self
    {
        ServerBuilder {
            attributes: att::server::ServerAttributes::new()
        }
    }

    /// Create a service constructor
    pub fn new_service_constructor<'a>(&'a mut self, service_type: UUID, is_primary: bool)
    -> ServiceBuilder<'a>
    {
        ServiceBuilder::new(&mut self.attributes, service_type, is_primary)
    }

    /// Make an server
    ///
    /// Construct an server from the server builder.
    pub fn make_server<C,Mtu>(self, connection_channel: C, server_mtu: Mtu)
    -> att::server::Server<C>
    where C: l2cap::ConnectionChannel,
          Mtu: Into<Option<u16>>
    {
        att::server::Server::new(connection_channel, server_mtu.into(), Some(self.attributes))
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
