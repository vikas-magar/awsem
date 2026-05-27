#[derive(Clone, Debug)]
pub struct AwsConfig {
    pub region: String,
    pub account_id: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub default_pool_id: String,
    pub jwt_issuer_url: String,
}

impl AwsConfig {
    pub fn new(region: &str, account_id: &str, access_key_id: &str, secret_access_key: &str, default_pool_id: &str) -> Self {
        let pool = if default_pool_id.is_empty() { format!("{region}_default") } else { default_pool_id.to_string() };
        Self {
            region: region.to_string(),
            account_id: account_id.to_string(),
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret_access_key.to_string(),
            default_pool_id: pool.clone(),
            jwt_issuer_url: format!("https://cognito-idp.{region}.amazonaws.com/{pool}"),
        }
    }

    pub fn partition(&self) -> &str { "aws" }
    pub fn lambda_arn(&self, name: &str) -> String { format!("arn:{}:lambda:{}:{}:function:{name}", self.partition(), self.region, self.account_id) }
    pub fn cognito_pool_arn(&self, pool_id: &str) -> String { format!("arn:{}:cognito-idp:{}:{}:userpool/{pool_id}", self.partition(), self.region, self.account_id) }
    pub fn secrets_arn(&self, name: &str) -> String { format!("arn:{}:secretsmanager:{}:{}:secret:{name}", self.partition(), self.region, self.account_id) }
    pub fn emr_vc_arn(&self, vc_id: &str) -> String { format!("arn:{}:emr-containers:{}:{}:/virtualclusters/{vc_id}", self.partition(), self.region, self.account_id) }
    pub fn emr_job_arn(&self, vc_id: &str, jr_id: &str) -> String { format!("arn:{}:emr-containers:{}:{}:/virtualclusters/{vc_id}/jobruns/{jr_id}", self.partition(), self.region, self.account_id) }
    pub fn emr_cluster_arn(&self, jf: &str) -> String { format!("arn:{}:emr:{}:{}:cluster/{jf}", self.partition(), self.region, self.account_id) }
    pub fn s3_bucket_arn(&self, bucket: &str) -> String { format!("arn:{}:s3:::{bucket}", self.partition()) }
    pub fn iam_role_arn(&self, name: &str) -> String { format!("arn:{}:iam::{}:role/{name}", self.partition(), self.account_id) }
}
