use aws_credential_types::Credentials;
use std::time::Duration;

#[derive(Clone)]
pub struct AwsClients {
    pub s3: aws_sdk_s3::Client,
    pub cognito: aws_sdk_cognitoidentityprovider::Client,
    pub secrets: aws_sdk_secretsmanager::Client,
    pub lambda: aws_sdk_lambda::Client,
    pub emr: aws_sdk_emrcontainers::Client,
    pub endpoint: String,
}

impl AwsClients {
    pub async fn new(endpoint: &str) -> Self {
        let creds = Credentials::new("awsem", "awsem", None, None, "awsem-tui");
        let ep = endpoint.trim_end_matches('/');

        let shared = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region("us-east-1")
            .credentials_provider(creds)
            .load()
            .await;

        let s3 = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::from(&shared)
                .to_builder()
                .endpoint_url(ep)
                .force_path_style(true)
                .build(),
        );

        let cognito = aws_sdk_cognitoidentityprovider::Client::from_conf(
            aws_sdk_cognitoidentityprovider::Config::from(&shared)
                .to_builder()
                .endpoint_url(ep)
                .build(),
        );

        let secrets = aws_sdk_secretsmanager::Client::from_conf(
            aws_sdk_secretsmanager::Config::from(&shared)
                .to_builder()
                .endpoint_url(ep)
                .build(),
        );

        let lambda = aws_sdk_lambda::Client::from_conf(
            aws_sdk_lambda::Config::from(&shared)
                .to_builder()
                .endpoint_url(ep)
                .build(),
        );

        let emr = aws_sdk_emrcontainers::Client::from_conf(
            aws_sdk_emrcontainers::Config::from(&shared)
                .to_builder()
                .endpoint_url(ep)
                .build(),
        );

        Self { s3, cognito, secrets, lambda, emr, endpoint: ep.to_string() }
    }

    pub async fn check_health(&self) -> bool {
        tokio::time::timeout(Duration::from_secs(2), async {
            tokio::net::TcpStream::connect(
                self.endpoint
                    .trim_start_matches("http://")
                    .trim_start_matches("https://"),
            )
            .await
            .is_ok()
        })
        .await
        .unwrap_or(false)
    }
}
