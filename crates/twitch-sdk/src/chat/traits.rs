pub(crate) trait Auth {
    async fn get_access_token(&self) -> String;
    async fn get_token_type(&self) -> String;
}
