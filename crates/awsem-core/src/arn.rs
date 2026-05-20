#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Arn {
    pub partition: String,
    pub service: String,
    pub region: String,
    pub account: String,
    pub resource: String,
}

impl Arn {
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.splitn(6, ':').collect();
        if parts.len() != 6 || parts[0] != "arn" {
            return Err("Invalid ARN format".into());
        }
        Ok(Self {
            partition: parts[1].into(),
            service: parts[2].into(),
            region: parts[3].into(),
            account: parts[4].into(),
            resource: parts[5].into(),
        })
    }

}

impl std::fmt::Display for Arn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "arn:{}:{}:{}:{}:{}",
            self.partition, self.service, self.region, self.account, self.resource)
    }
}

// Builder functions
pub fn s3_bucket(account: &str, name: &str) -> Arn {
    Arn { partition: "aws".into(), service: "s3".into(), region: "us-east-1".into(), account: account.into(), resource: format!("bucket/{name}") }
}

pub fn cognito_pool(account: &str, region: &str, pool_id: &str) -> Arn {
    Arn { partition: "aws".into(), service: "cognito-idp".into(), region: region.into(), account: account.into(), resource: format!("userpool/{pool_id}") }
}

pub fn secret(account: &str, name: &str) -> Arn {
    Arn { partition: "aws".into(), service: "secretsmanager".into(), region: "us-east-1".into(), account: account.into(), resource: format!("secret:{name}") }
}

pub fn emr_virtual_cluster(account: &str, id: &str) -> Arn {
    Arn { partition: "aws".into(), service: "emr-containers".into(), region: "us-east-1".into(), account: account.into(), resource: format!("/virtualclusters/{id}") }
}

pub fn emr_job_run(account: &str, vc_id: &str, jr_id: &str) -> Arn {
    Arn { partition: "aws".into(), service: "emr-containers".into(), region: "us-east-1".into(), account: account.into(), resource: format!("/virtualclusters/{vc_id}/jobruns/{jr_id}") }
}

pub fn lambda_fn(account: &str, name: &str) -> Arn {
    Arn { partition: "aws".into(), service: "lambda".into(), region: "us-east-1".into(), account: account.into(), resource: format!("function:{name}") }
}
