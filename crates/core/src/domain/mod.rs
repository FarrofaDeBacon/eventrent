pub mod audit;
pub mod branding;
pub mod config;
pub mod customer;
pub mod document;
pub mod permission;
pub mod role;
pub mod tenant;
pub mod user;
pub mod validation;
pub mod venue;

pub use audit::AuditLogEntry;
pub use branding::{BrandingValidator, PublicBranding, TenantBranding, UpdateBrandingInput};
pub use config::{AppConfig, InitialSetupInput, InitialSetupResult, OperationMode};
pub use customer::{
    build_customer_search_text, build_name_sort, normalize_for_search, remove_diacritics,
    validate_addresses, validate_contacts, validate_customer_common_fields,
    validate_roles_and_categories, CreateAddressInput, CreateContactInput, CreateCustomerInput,
    Customer, CustomerAddress, CustomerContact, CustomerCursor, CustomerFilter, CustomerPage,
    CustomerRole, CustomerRoleFilter, CustomerWithDetails, PersonType, SupplierCategory,
    UpdateAddressInput, UpdateContactInput, UpdateCustomerInput,
};
pub use document::{
    format_cnpj, format_cpf, mask_document_for_audit, mask_email_for_audit,
    mask_individual_name_for_audit, mask_phone_for_audit, normalize_document, parse_document,
    parse_document_with_type, validate_cnpj, validate_cpf, validate_foreign_document,
    DocumentNumber, DocumentType,
};
pub use permission::*;
pub use role::{Role, RoleWithPermissions};
pub use tenant::Tenant;
pub use user::{
    CreateUserInput, LoginInput, LoginPolicy, RecoverPasswordInput, RecoverPasswordResult,
    ResetPasswordInput, SessionInfo, UpdateUserInput, UserRecord, UserWithRole,
};
pub use validation::{
    normalize_and_validate_phone, normalize_and_validate_postal_code, validate_brazilian_state,
    validate_country_code, validate_email, validate_user_name, BRAZILIAN_STATES,
};
pub use venue::{
    build_venue_name_sort, build_venue_search_text, validate_venue_contacts, validate_venue_fields,
    validate_venue_spaces, CreateVenueInput, UpdateVenueInput, ValidatedVenueFields, Venue,
    VenueContact, VenueContactInput, VenueCursor, VenueFilter, VenueListItem, VenuePage,
    VenueSpace, VenueSpaceInput, VenueWithDetails, MAX_INTERNAL_TRANSFER_MINUTES,
    MAX_VENUE_CONTACTS, MAX_VENUE_NAME_LEN, MAX_VENUE_SPACES, MAX_VENUE_SPACE_NAME_LEN,
    MAX_VENUE_TECH_FIELD_LEN,
};
