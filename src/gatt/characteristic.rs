use alloc::{
    boxed::Box,
    vec::Vec,
    string::String,
    format,
};
use crate::{att, UUID};

/// Characteristic Properties
///
/// These are the properties that are part of the Characteristic Declaration
#[derive(Clone,Copy,PartialEq,PartialOrd,Eq,Ord,Debug)]
pub enum Properties {
    Broadcast,
    Read,
    WriteWithoutResponse,
    Write,
    Notify,
    Indicate,
    AuthenticatedSignedWrite,
    ExtendedProperties,
}

impl Properties {
    fn into_val(&self) -> u8 {
        match *self {
            Properties::Broadcast => 1 << 0,
            Properties::Read => 1 << 1,
            Properties::WriteWithoutResponse => 1 << 2,
            Properties::Write => 1 << 3,
            Properties::Notify => 1 << 4,
            Properties::Indicate => 1 << 5,
            Properties::AuthenticatedSignedWrite => 1 << 6,
            Properties::ExtendedProperties => 1 << 7,
        }
    }

    fn into_bit_field(properties: &[Self]) -> u8 {
        properties.iter().fold( 0u8, |u, p| u | p.into_val() )
    }

    fn from_bit_field(field: u8) -> Box<[Self]> {
        let from_raw = |raw| {
            match raw {
                0x01 => Properties::Broadcast,
                0x02 => Properties::Read,
                0x04 => Properties::WriteWithoutResponse,
                0x08 => Properties::Write,
                0x10 => Properties::Notify,
                0x20 => Properties::Indicate,
                0x40 => Properties::AuthenticatedSignedWrite,
                0x80 => Properties::ExtendedProperties,
                _ => panic!("Impossibile bit field")
            }
        };

        let mut vec = Vec::new();

        for shift in 0..8 {
            vec.push(from_raw( field & (1 << shift)))
        }

        vec.into_boxed_slice()
    }
}

impl att::TransferFormat for Box<[Properties]> {
    fn from(raw: &[u8]) -> Result<Self, att::TransferFormatError> {
        if raw.len() == 1 {
            Ok( Properties::from_bit_field(raw[0]) )
        } else {
            Err( att::TransferFormatError::bad_size(stringify!(Box<[Properties]>), 1, raw.len()) )
        }
    }

    fn into(&self) -> Box<[u8]> {
        let cp: &[u8] = &[Properties::into_bit_field(self)];

        cp.into()
    }
}

impl att::TransferFormatSize for Box<[Properties]> {
    const SIZE: usize = 1;
}

struct Declaration {
    properties: Box<[Properties]>,
    value_handle: u16,
    uuid: UUID,
}

impl att::TransferFormat for Declaration {
    fn from(raw: &[u8]) -> Result<Self, att::TransferFormatError> {
        // The implementation of TransferFormat for UUID will check if the length is good for
        // a 128 bit UUID
        if raw.len() >= 6 {
            Ok( Declaration {
                properties: att::TransferFormat::from( &raw[..1] )?,
                value_handle: att::TransferFormat::from( &raw[1..3] )?,
                uuid: att::TransferFormat::from( &raw[3..])?,
            })
        } else {
            Err( att::TransferFormatError::bad_min_size(stringify!(Declaration), 6, raw.len()) )
        }
    }

    fn into(&self) -> Box<[u8]> {
        let mut v = Vec::new();

        v.extend_from_slice( &att::TransferFormat::into(&self.properties) );
        v.extend_from_slice( &att::TransferFormat::into(&self.value_handle) );
        v.extend_from_slice( &att::TransferFormat::into(&self.uuid) );

        v.into_boxed_slice()
    }
}

impl Declaration {

    const TYPE: UUID = UUID::from_u16(0x2803);

    const PERMISSIONS: &'static [att::AttributePermissions] = &[att::AttributePermissions::Read];
}

struct ValueDeclaration<V> where V: ?Sized {
    att_type: UUID,
    value: Box<V>,
    permissions: Vec<att::AttributePermissions>,
}

pub enum ExtendedProperties {
    ReliableWrite,
    WritableAuxiliaries
}

impl att::TransferFormat for ExtendedProperties {
    fn from(raw: &[u8]) -> Result<Self, att::TransferFormatError> {
        if raw.len() == 2 {
            match <u16>::from_le_bytes( [ raw[0], raw[1] ] ) {
                0x1 => Ok( ExtendedProperties::ReliableWrite ),
                0x2 => Ok( ExtendedProperties::WritableAuxiliaries ),
                e @ _ => Err( att::TransferFormatError::from(
                    format!("Unknown extended property '{}'", e)) )
            }
        } else {
            Err( att::TransferFormatError::bad_size(stringify!(ExtendedProperties), 2, raw.len()) )
        }
    }

    fn into(&self) -> Box<[u8]> {
        let val = match *self {
            ExtendedProperties::ReliableWrite => 0x1,
            ExtendedProperties::WritableAuxiliaries => 0x2,
        };

        From::<&[u8]>::from( &[val] )
    }
}

impl att::TransferFormatSize for ExtendedProperties {
    const SIZE: usize = 1;
}

impl ExtendedProperties {
    const TYPE: UUID = UUID::from_u16(0x2803);

    const PERMISSIONS: &'static [att::AttributePermissions] = &[att::AttributePermissions::Read];
}

pub struct UserDescription {
    value: String,
    permissions: Vec<att::AttributePermissions>
}

impl UserDescription {
    const TYPE: UUID = UUID::from_u16(0x2901);

    pub fn new<D>(description: D, permissions: Vec<att::AttributePermissions>) -> Self
    where D: Into<String>
    {
        UserDescription {
            value: description.into(),
            permissions: permissions
        }
    }
}

pub enum ClientConfiguration {
    Notification,
    Indication
}

impl att::TransferFormat for ClientConfiguration {
    fn from(raw: &[u8]) -> Result<Self, att::TransferFormatError> {
        if raw.len() == 2 {
            match <u16>::from_le_bytes( [ raw[0], raw[1] ] ) {
                0x1 => Ok( ClientConfiguration::Notification ),
                0x2 => Ok( ClientConfiguration::Indication ),
                e @ _ => Err( att::TransferFormatError::from(
                    format!("Unknown Client Configuration '{}'", e)))
            }
        } else {
            Err( att::TransferFormatError::bad_size(stringify!(ClientConfiguration), 2, raw.len()) )
        }
    }

    fn into(&self) -> Box<[u8]> {
        let val = match *self {
            ClientConfiguration::Notification => 0x1,
            ClientConfiguration::Indication => 0x2,
        };

        From::<&[u8]>::from( &[val] )
    }
}

impl att::TransferFormatSize for ClientConfiguration {
    const SIZE: usize = 1;
}

impl ClientConfiguration {
    const TYPE: UUID = UUID::from_u16(2903);

    const PERMISSIONS: &'static [att::AttributePermissions] = &[
        att::AttributePermissions::Read,
        att::AttributePermissions::Authentication(att::AttributeRestriction::Write),
        att::AttributePermissions::Authorization(att::AttributeRestriction::Write)
    ];
}

pub enum ServerConfiguration {
    Broadcast
}

impl att::TransferFormat for ServerConfiguration {
    fn from(raw: &[u8]) -> Result<Self, att::TransferFormatError> {
        if raw.len() == 2 {
            match <u16>::from_le_bytes( [ raw[0], raw[1] ] ) {
                0x1 => Ok( ServerConfiguration::Broadcast ),
                e @ _ => Err( att::TransferFormatError::from(
                    format!("Unknown server configuration: '{}'", e)))
            }
        } else {
            Err( att::TransferFormatError::bad_size(stringify!(ServerConfiguration), 2, raw.len()) )
        }
    }

    fn into(&self) -> Box<[u8]> {
        let val = match *self {
            ServerConfiguration::Broadcast => 0x1,
        };

        From::<&[u8]>::from( &[val] )
    }
}

impl att::TransferFormatSize for ServerConfiguration {
    const SIZE: usize = 1;
}

impl ServerConfiguration {
    const TYPE: UUID = UUID::from_u16(2903);

    const PERMISSIONS: &'static [att::AttributePermissions] = &[
        att::AttributePermissions::Read,
        att::AttributePermissions::Authentication(att::AttributeRestriction::Write),
        att::AttributePermissions::Authorization(att::AttributeRestriction::Write)
    ];
}

pub struct CharacteristicBuilder<'a, V> where V: ?Sized {
    characteristic_adder: super::CharacteristicAdder<'a>,
    declaration: Declaration,
    value_decl: ValueDeclaration<V>,
    ext_prop: Option<Vec<ExtendedProperties>>,
    user_desc: Option<UserDescription>,
    client_cfg: Option<Vec<ClientConfiguration>>,
    server_cfg: Option<Vec<ServerConfiguration>>,
}

impl<'a, V> CharacteristicBuilder<'a, V>
where Box<V>: att::TransferFormat + Unpin + 'static,
           V: ?Sized
{
    pub(super) fn new(
        characteristic_adder: super::CharacteristicAdder<'a>,
        properties: Vec<Properties>,
        uuid: UUID,
        value: Box<V>,
        value_permissions: Vec<att::AttributePermissions>
    ) -> CharacteristicBuilder<V>
    {
        CharacteristicBuilder {
            characteristic_adder: characteristic_adder,
            declaration: Declaration {
                properties: properties.into_boxed_slice(),
                value_handle: 0,
                uuid: uuid
            },
            value_decl: ValueDeclaration {
                att_type: uuid,
                value: value,
                permissions: value_permissions,
            },
            ext_prop: None,
            user_desc: None,
            client_cfg: None,
            server_cfg: None,
        }
    }

    /// Instruct the builder to create a `Extended Properties` characteristic descriptor
    /// upon building the characteristic unless the value of `extended_properties` is `None`.
    #[inline]
    pub fn set_extended_properties<E>( mut self, extended_properties: E) -> Self
    where E: Into<Option<Vec<ExtendedProperties>>>
    {
        self.ext_prop = extended_properties.into();
        self
    }

    /// Instruct the builder to create a `User Description` characteristic descriptor
    /// upon building the characteristic unless the value of `user_description` is `None`.
    #[inline]
    pub fn set_user_description<D>( mut self, user_description: D) -> Self
    where D: Into<Option<UserDescription>>
    {
        self.user_desc = user_description.into();
        self
    }

    /// Instruct the builder to create a `Client Configuration` characteristic descriptor
    /// upon building the characteristic unless the value of `client_cfg` is `None`.
    #[inline]
    pub fn set_client_configuration<C>( mut self, client_cfg: C) -> Self
    where C: Into<Option<Vec<ClientConfiguration>>>
    {
        self.client_cfg = client_cfg.into();
        self
    }

    /// Instruct the builder to create a `Server Configuration` characteristic descriptor
    /// upon building the characteristic unless the value of `server_cfg` is `None`.
    #[inline]
    pub fn set_server_configuration<C>( mut self, server_cfg: C) -> Self
    where C: Into<Option<Vec<ServerConfiguration>>>
    {
        self.server_cfg = server_cfg.into();
        self
    }

    /// Finish constructing the Characteristic
    ///
    /// This will return the CharacteristicAdder that was used to make this CharacteristicBuilder.
    ///
    pub fn finish_characteristic(mut self) -> super::CharacteristicAdder<'a>
    {
        use att::Attribute;

        let attributes = &mut self.characteristic_adder.service_builder.server_builder.attributes;

        // The value handle will be the handle after the declaration
        self.declaration.value_handle = attributes.next_handle() + 1;

        let declaration = Attribute::new(
            Declaration::TYPE,
            Declaration::PERMISSIONS.into(),
            self.declaration
        );

        attributes.push(declaration);

        let value = Attribute::new(
            self.value_decl.att_type,
            self.value_decl.permissions,
            self.value_decl.value
        );

        // last_attr is handle value of the added attribute
        let mut last_attr = attributes.push(value);

        if let Some(ext) = self.ext_prop.take() {
            last_attr = attributes.push(
                Attribute::new(
                    ExtendedProperties::TYPE,
                    ExtendedProperties::PERMISSIONS.into(),
                    ext.into_boxed_slice(),
                )
            );
        }

        if let Some(desc) = self.user_desc.take() {
            last_attr = attributes.push(
                Attribute::new(
                    UserDescription::TYPE,
                    desc.permissions,
                    desc.value
                )
            );
        }

        if let Some(client_cfg) = self.client_cfg.take() {
            last_attr = attributes.push(
                Attribute::new(
                    ClientConfiguration::TYPE,
                    ClientConfiguration::PERMISSIONS.into(),
                    client_cfg.into_boxed_slice()
                )
            );
        }

        if let Some(server_cfg) = self.server_cfg.take() {
            last_attr = attributes.push(
                Attribute::new(
                    ServerConfiguration::TYPE,
                    ServerConfiguration::PERMISSIONS.into(),
                    server_cfg.into_boxed_slice()
                )
            );
        }

        self.characteristic_adder.end_group_handle =last_attr;

        self.characteristic_adder
    }
}
