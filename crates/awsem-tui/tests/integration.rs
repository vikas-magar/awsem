use aws_sdk_cognitoidentityprovider as cog;
use aws_sdk_emrcontainers as emr;
use aws_sdk_lambda as lam;
use aws_sdk_secretsmanager as sec;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const PID: &str = "us-east-1_default";
const BINARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/debug/awsem");

struct TestServer {
    child: Option<Child>,
    dir: String,
    port: u16,
}

impl TestServer {
    fn start() -> Self {
        let port: u16 = 45679 + (std::process::id() % 1000) as u16;
        let dir = format!("/tmp/awsem-test-{}", std::process::id());
        let db = format!("{dir}/test.db");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut child = Command::new(BINARY)
            .args(["--port", &port.to_string(), "--db-path", &db, "--data-dir", &dir, "--log-dir", &dir])
            .args(["--jwt-secret", "test-secret", "--no-s3", "true", "--no-emr", "true"])
            .stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().unwrap_or_else(|e| panic!("Failed to start awsem at {BINARY}: {e}"));
        let start = Instant::now();
        loop {
            if child.try_wait().ok().flatten().is_some() { panic!("Server exited during startup"); }
            if TcpStream::connect_timeout(&format!("127.0.0.1:{port}").parse().unwrap(), Duration::from_secs(1)).is_ok() { break; }
            if start.elapsed().as_secs() > 25 { panic!("Server startup timeout on port {port}"); }
            std::thread::sleep(Duration::from_millis(200));
        }
        std::thread::sleep(Duration::from_millis(500));
        Self { child: Some(child), dir, port }
    }

    fn ep(&self) -> String { format!("http://127.0.0.1:{}", self.port) }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn make_clients(ep: &str) -> (cog::Client, sec::Client, lam::Client, emr::Client) {
    let creds = aws_credential_types::Credentials::new("awsem", "awsem", None, None, "test");
    let cfg = aws_config::defaults(aws_config::BehaviorVersion::latest()).region("us-east-1").credentials_provider(creds).load().await;
    let cognito = cog::Client::from_conf(cog::Config::from(&cfg).to_builder().endpoint_url(ep).build());
    let secrets = sec::Client::from_conf(sec::Config::from(&cfg).to_builder().endpoint_url(ep).build());
    let lambda = lam::Client::from_conf(lam::Config::from(&cfg).to_builder().endpoint_url(ep).build());
    let emr = emr::Client::from_conf(emr::Config::from(&cfg).to_builder().endpoint_url(ep).build());
    (cognito, secrets, lambda, emr)
}

#[tokio::test]
async fn test_all_services() {
    let svr = TestServer::start();
    let (cognito, secrets, lambda, emr) = make_clients(&svr.ep()).await;

    // --- Cognito ---
    let username = "test-user";
    cognito.admin_create_user().user_pool_id(PID).username(username).temporary_password("TestPass123!").send().await.expect("AdminCreateUser");
    let users = cognito.list_users().user_pool_id(PID).send().await.expect("ListUsers");
    assert!(users.users().iter().any(|u| u.username() == Some(username)));
    let user = cognito.admin_get_user().user_pool_id(PID).username(username).send().await.expect("AdminGetUser");
    assert_eq!(user.username(), username);
    cognito.admin_delete_user().user_pool_id(PID).username(username).send().await.expect("AdminDeleteUser");
    let users = cognito.list_users().user_pool_id(PID).send().await.expect("ListUsers after delete");
    assert!(!users.users().iter().any(|u| u.username() == Some(username)));

    // SignUp + Confirm + InitiateAuth
    cognito.sign_up().client_id("test").username("auth-user").password("AuthPass1!").send().await.expect("SignUp");
    cognito.confirm_sign_up().client_id("test").username("auth-user").confirmation_code("000000").send().await.expect("ConfirmSignUp");
    let auth = cognito.initiate_auth().auth_flow(cog::types::AuthFlowType::UserPasswordAuth).auth_parameters("USERNAME", "auth-user").auth_parameters("PASSWORD", "AuthPass1!").client_id("test").send().await.expect("InitiateAuth");
    assert!(auth.authentication_result().is_some());
    cognito.admin_delete_user().user_pool_id(PID).username("auth-user").send().await.expect("AdminDeleteUser cleanup");

    // --- Secrets Manager ---
    secrets.create_secret().name("test/my-secret").secret_string("hello-world").send().await.expect("CreateSecret");
    let list = secrets.list_secrets().send().await.expect("ListSecrets");
    assert!(list.secret_list().iter().any(|s| s.name() == Some("test/my-secret")));
    let got = secrets.get_secret_value().secret_id("test/my-secret").send().await.expect("GetSecretValue");
    assert_eq!(got.secret_string(), Some("hello-world"));
    secrets.put_secret_value().secret_id("test/my-secret").secret_string("updated-value").send().await.expect("PutSecretValue");
    let got = secrets.get_secret_value().secret_id("test/my-secret").send().await.expect("GetSecretValue updated");
    assert_eq!(got.secret_string(), Some("updated-value"));
    secrets.delete_secret().secret_id("test/my-secret").force_delete_without_recovery(true).send().await.expect("DeleteSecret");
    let list = secrets.list_secrets().send().await.expect("ListSecrets after delete");
    assert!(!list.secret_list().iter().any(|s| s.name() == Some("test/my-secret")));

    // --- Lambda ---
    let code = lam::types::FunctionCode::builder().zip_file(lam::primitives::Blob::new(vec![])).build();
    lambda.create_function().function_name("test-func").role("arn:aws:iam::000000000000:role/test").runtime(lam::types::Runtime::Providedal2023).handler("index.handler").code(code).send().await.expect("CreateFunction");
    let list = lambda.list_functions().send().await.expect("ListFunctions");
    assert!(list.functions().iter().any(|f| f.function_name() == Some("test-func")));
    lambda.delete_function().function_name("test-func").send().await.expect("DeleteFunction");
    let list = lambda.list_functions().send().await.expect("ListFunctions after delete");
    assert!(!list.functions().iter().any(|f| f.function_name() == Some("test-func")));

    // --- EMR ---
    let eks_info = emr::types::EksInfo::builder().namespace("awsem").build();
    let container_info = emr::types::ContainerInfo::EksInfo(eks_info);
    let provider = emr::types::ContainerProvider::builder().id("test-eks").r#type(emr::types::ContainerProviderType::Eks).info(container_info).build().unwrap();
    let vc = emr.create_virtual_cluster().name("test-vc").container_provider(provider).send().await.expect("CreateVirtualCluster");
    let vc_id = vc.id().unwrap_or("");
    assert!(!vc_id.is_empty());
    let list = emr.list_virtual_clusters().send().await.expect("ListVirtualClusters");
    assert!(list.virtual_clusters().iter().any(|v| v.name() == Some("test-vc")));
    emr.delete_virtual_cluster().id(vc_id).send().await.expect("DeleteVirtualCluster");
    let list = emr.list_virtual_clusters().send().await.expect("ListVirtualClusters after delete");
    assert!(!list.virtual_clusters().iter().any(|v| v.name() == Some("test-vc")));
}
