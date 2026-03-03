use std::sync::Arc;

pub(crate) trait Auth {
    fn get_access_token(&self) -> Option<Arc<String>>;
    fn get_token_type(&self) -> Option<Arc<String>>;
    fn get_client_id(&self) -> Arc<String>;
}
