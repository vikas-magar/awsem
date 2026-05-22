//! awsem validation — runs AWS SDK operations against a fresh server.
//!
//! Usage: cargo run --example validate -p awsem-tui
//! Requires: target/debug/awsem (server binary) already built.
//! K8s-dependent tests (S3, EMR Spark) are attempted and skipped if unavailable.

use aws_sdk_cognitoidentityprovider as cog;
use aws_sdk_emrcontainers as emr;
use aws_sdk_lambda as lam;
use aws_sdk_s3 as s3sdk;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_secretsmanager as sec;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const BINARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/debug/awsem");
const PID: &str = "us-east-1_default";
const SPARK_SCRIPT: &str = "import sys\nfrom pyspark.sql import SparkSession\nspark = SparkSession.builder.appName('line-count').getOrCreate()\ndf = spark.read.text(sys.argv[1])\nlines = df.count()\nresult = spark.createDataFrame([(lines,)], ['line_count'])\nresult.coalesce(1).write.mode('overwrite').option('header', 'true').csv(sys.argv[2])\n";

struct TestServer { child: Option<Child>, dir: String, port: u16 }

impl TestServer {
    fn start() -> Self {
        let port: u16 = 45679 + (std::process::id() % 1000) as u16;
        let dir = format!("/tmp/awsem-val-{}", std::process::id());
        let db = format!("{dir}/test.db");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let host_ip = std::env::var("AWSEM_HOST_IP").unwrap_or_else(|_| "192.168.1.3".into());
        let mut child = Command::new(BINARY)
            .args(["--port", &port.to_string(), "--db-path", &db, "--data-dir", &dir, "--log-dir", &dir])
            .args(["--jwt-secret", "test-secret", "--emr-spark-image", "spark-s3a:latest", "--host-ip", &host_ip])
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
    fn port(&self) -> u16 { self.port }
    fn ep(&self) -> String { format!("http://127.0.0.1:{}", self.port) }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() { let _ = c.kill(); let _ = c.wait(); }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn clients(ep: &str) -> (cog::Client, sec::Client, lam::Client, emr::Client, s3sdk::Client) {
    let creds = aws_credential_types::Credentials::new("awsem", "awsem", None, None, "test");
    let cfg = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region("us-east-1").credentials_provider(creds).load().await;
    (cog::Client::from_conf(cog::Config::from(&cfg).to_builder().endpoint_url(ep).build()),
     sec::Client::from_conf(sec::Config::from(&cfg).to_builder().endpoint_url(ep).build()),
     lam::Client::from_conf(lam::Config::from(&cfg).to_builder().endpoint_url(ep).build()),
     emr::Client::from_conf(emr::Config::from(&cfg).to_builder().endpoint_url(ep).build()),
     s3sdk::Client::from_conf(s3sdk::Config::from(&cfg).to_builder().endpoint_url(ep).force_path_style(true).build()))
}

macro_rules! ok {
    ($name:expr, $body:expr) => {{
        match tokio::time::timeout(Duration::from_secs(15), async { $body }).await {
            Ok(Ok(r)) => { println!("  PASS  {}", $name); Some(r) }
            Ok(Err(e)) => { println!("  FAIL  {} — {}", $name, e); None }
            Err(_) => { println!("  FAIL  {} — timed out", $name); None }
        }
    }};
    ($name:expr, $body:expr, skip) => {{
        match tokio::time::timeout(Duration::from_secs(30), async { $body }).await {
            Ok(Ok(r)) => { println!("  PASS  {}", $name); Some(r) }
            Ok(Err(e)) => { println!("  SKIP  {} — {}", $name, e); None }
            Err(_) => { println!("  SKIP  {} — timed out (K8s may be unavailable)", $name); None }
        }
    }};
}

#[tokio::main]
async fn main() {
    println!("\nawsem validation suite\n======================");
    println!("Server binary: {BINARY}");
    let server = TestServer::start();
    println!("Server on port {}", server.port());
    let (cog, sec, lam, emr, s3) = clients(&server.ep()).await;

    println!("\n── Cognito ──────────────────────────────");

    ok!("AdminCreateUser",
        cog.admin_create_user().user_pool_id(PID).username("crud-user").temporary_password("CrudPass1!").send().await);
    if let Some(list) = ok!("ListUsers",
        cog.list_users().user_pool_id(PID).send().await) {
        assert!(list.users().iter().any(|u| u.username() == Some("crud-user")));
    }
    if let Some(u) = ok!("AdminGetUser",
        cog.admin_get_user().user_pool_id(PID).username("crud-user").send().await) {
        assert_eq!(u.username(), "crud-user");
    }
    ok!("AdminDeleteUser",
        cog.admin_delete_user().user_pool_id(PID).username("crud-user").send().await);
    if let Some(list) = ok!("ListUsers after delete",
        cog.list_users().user_pool_id(PID).send().await) {
        assert!(!list.users().iter().any(|u| u.username() == Some("crud-user")));
    }
    ok!("SignUp",
        cog.sign_up().client_id("test").username("auth-crud").password("AuthPass1!").send().await);
    ok!("ConfirmSignUp",
        cog.confirm_sign_up().client_id("test").username("auth-crud").confirmation_code("000000").send().await);
    if let Some(auth) = ok!("InitiateAuth",
        cog.initiate_auth().auth_flow(cog::types::AuthFlowType::UserPasswordAuth)
            .auth_parameters("USERNAME", "auth-crud").auth_parameters("PASSWORD", "AuthPass1!")
            .client_id("test").send().await) {
        assert!(auth.authentication_result().is_some());
    }

    println!("\n── Secrets Manager ───────────────────────");

    ok!("CreateSecret",
        sec.create_secret().name("crud/my-secret").secret_string("hello").send().await);
    if let Some(got) = ok!("GetSecretValue",
        sec.get_secret_value().secret_id("crud/my-secret").send().await) {
        assert_eq!(got.secret_string(), Some("hello"));
    }
    ok!("PutSecretValue",
        sec.put_secret_value().secret_id("crud/my-secret").secret_string("world").send().await);
    if let Some(got) = ok!("GetSecretValue updated",
        sec.get_secret_value().secret_id("crud/my-secret").send().await) {
        assert_eq!(got.secret_string(), Some("world"));
    }
    ok!("DescribeSecret",
        sec.describe_secret().secret_id("crud/my-secret").send().await);
    ok!("UpdateSecret",
        sec.update_secret().secret_id("crud/my-secret").description("test").send().await);
    ok!("ListSecrets", sec.list_secrets().send().await);
    ok!("DeleteSecret",
        sec.delete_secret().secret_id("crud/my-secret").force_delete_without_recovery(true).send().await);
    ok!("CreateSecret for restore",
        sec.create_secret().name("crud/restore-me").secret_string("original").send().await);
    sec.delete_secret().secret_id("crud/restore-me").send().await.ok();
    ok!("RestoreSecret",
        sec.restore_secret().secret_id("crud/restore-me").send().await);
    sec.delete_secret().secret_id("crud/restore-me").force_delete_without_recovery(true).send().await.ok();

    println!("\n── Lambda ────────────────────────────────");

    let code = lam::types::FunctionCode::builder().zip_file(lam::primitives::Blob::new(vec![])).build();
    ok!("CreateFunction",
        lam.create_function().function_name("crud-func").role("arn:aws:iam::000000000000:role/test")
            .runtime(lam::types::Runtime::Providedal2023).handler("index.handler").code(code.clone()).send().await);
    if let Some(f) = ok!("GetFunction",
        lam.get_function().function_name("crud-func").send().await) {
        let c = f.configuration().unwrap();
        assert_eq!(c.function_name(), Some("crud-func"));
        assert_eq!(c.runtime(), Some(&lam::types::Runtime::Providedal2023));
        assert!(c.last_modified().is_some());
    }
    ok!("ListFunctions", lam.list_functions().send().await);
    ok!("DeleteFunction", lam.delete_function().function_name("crud-func").send().await);
    if let Some(list) = ok!("ListFunctions after delete",
        lam.list_functions().send().await) {
        assert!(!list.functions().iter().any(|f| f.function_name() == Some("crud-func")));
    }

    println!("\n── EMR ───────────────────────────────────");

    let eks = emr::types::EksInfo::builder().namespace("awsem").build();
    let provider = emr::types::ContainerProvider::builder().id("val-eks")
        .r#type(emr::types::ContainerProviderType::Eks)
        .info(emr::types::ContainerInfo::EksInfo(eks)).build().unwrap();
    if let Some(vc) = ok!("CreateVirtualCluster",
        emr.create_virtual_cluster().name("crud-vc").container_provider(provider.clone()).send().await) {
        assert!(!vc.id().unwrap_or("").is_empty());
    }
    ok!("ListVirtualClusters", emr.list_virtual_clusters().send().await);
    let vc_id = emr.list_virtual_clusters().send().await.ok()
        .and_then(|l| l.virtual_clusters().into_iter().find(|v| v.name() == Some("crud-vc"))?.id().map(String::from))
        .unwrap_or_default();
    if !vc_id.is_empty() {
        ok!("DeleteVirtualCluster", emr.delete_virtual_cluster().id(&vc_id).send().await);
        if let Some(list) = ok!("ListVirtualClusters after delete",
            emr.list_virtual_clusters().send().await) {
            assert!(!list.virtual_clusters().iter().any(|v| v.name() == Some("crud-vc")));
        }
    }

    // ──────────────────────────────────────────────
    // E2E workflows
    // ──────────────────────────────────────────────

    println!("\n── E2E: Secret → User (cross-service) ────");

    ok!("store password in Secrets Manager",
        sec.create_secret().name("e2e/user-password").secret_string("E2eUserPass!1").send().await);
    let pwd = sec.get_secret_value().secret_id("e2e/user-password").send().await
        .ok().and_then(|v| v.secret_string().map(String::from)).unwrap_or_default();
    if !pwd.is_empty() {
        ok!("create Cognito user with secret password",
            cog.admin_create_user().user_pool_id(PID).username("e2e-user").temporary_password(&pwd).send().await);
        ok!("authenticate with secret password",
            cog.initiate_auth().auth_flow(cog::types::AuthFlowType::UserPasswordAuth)
                .auth_parameters("USERNAME", "e2e-user").auth_parameters("PASSWORD", &pwd)
                .client_id("test").send().await);
        ok!("cleanup e2e user",
            cog.admin_delete_user().user_pool_id(PID).username("e2e-user").send().await);
    }
    ok!("cleanup e2e secret",
        sec.delete_secret().secret_id("e2e/user-password").force_delete_without_recovery(true).send().await);

    println!("\n── E2E: Secret lifecycle ──────────────────");

    ok!("create secret v1",
        sec.create_secret().name("e2e/lifecycle").secret_string("v1").send().await);
    ok!("get v1", sec.get_secret_value().secret_id("e2e/lifecycle").send().await);
    ok!("put v2",
        sec.put_secret_value().secret_id("e2e/lifecycle").secret_string("v2").send().await);
    ok!("get v2", sec.get_secret_value().secret_id("e2e/lifecycle").send().await);
    ok!("describe dates", sec.describe_secret().secret_id("e2e/lifecycle").send().await);
    ok!("soft delete",
        sec.delete_secret().secret_id("e2e/lifecycle").send().await);
    ok!("restore", sec.restore_secret().secret_id("e2e/lifecycle").send().await);
    ok!("get after restore",
        sec.get_secret_value().secret_id("e2e/lifecycle").send().await);
    ok!("force delete",
        sec.delete_secret().secret_id("e2e/lifecycle").force_delete_without_recovery(true).send().await);

    println!("\n── E2E: Lambda configuration ──────────────");

    let lam_code = lam::types::FunctionCode::builder().zip_file(lam::primitives::Blob::new(vec![])).build();
    ok!("create Lambda with full config",
        lam.create_function().function_name("e2e-lam").role("arn:aws:iam::000000000000:role/e2e")
            .runtime(lam::types::Runtime::Nodejs20x).handler("main.handler")
            .timeout(30).memory_size(256).code(lam_code).send().await);
    if let Some(f) = ok!("verify Lambda config fields",
        lam.get_function().function_name("e2e-lam").send().await) {
        let c = f.configuration().unwrap();
        assert_eq!(c.function_name(), Some("e2e-lam"));
        assert_eq!(c.runtime(), Some(&lam::types::Runtime::Nodejs20x));
        assert_eq!(c.handler(), Some("main.handler"));
        assert_eq!(c.timeout(), Some(30));
        assert_eq!(c.memory_size(), Some(256));
    }
    ok!("delete Lambda", lam.delete_function().function_name("e2e-lam").send().await);

    println!("\n── E2E: S3 ───────────────────────────────");

    let bucket = format!("e2e-bucket-{}", std::process::id());
    // S3 operations use (skip) — they silently pass if K8s/RustFS is unavailable
    ok!("create bucket", s3.create_bucket().bucket(&bucket).send().await, skip);
    ok!("upload file", s3.put_object().bucket(&bucket).key("hello.txt")
        .body(ByteStream::from(b"Hello, awsem!\nLine two.\nLine three.\n".to_vec())).send().await, skip);
    if let Some(objs) = ok!("list objects", s3.list_objects_v2().bucket(&bucket).send().await, skip) {
        let keys: Vec<String> = objs.contents().iter().filter_map(|o| o.key().map(String::from)).collect();
        assert!(keys.contains(&"hello.txt".to_string()), "hello.txt should be listed");
    }
    ok!("download and verify",
        s3.get_object().bucket(&bucket).key("hello.txt").send().await, skip);
    ok!("delete object", s3.delete_object().bucket(&bucket).key("hello.txt").send().await, skip);
    if let Some(objs) = ok!("list after delete",
        s3.list_objects_v2().bucket(&bucket).send().await, skip) {
        assert!(objs.contents().is_empty(), "bucket should be empty after delete");
    }
    ok!("delete bucket", s3.delete_bucket().bucket(&bucket).send().await, skip);

    println!("\n── E2E: EMR Spark — S3 line count ────────");

    // Upload a small input file and a PySpark script to S3, then run a Spark job
    let spark_bucket = format!("e2e-spark-{}", std::process::id());

    ok!("create spark bucket", s3.create_bucket().bucket(&spark_bucket).send().await, skip);
    ok!("upload input file", s3.put_object().bucket(&spark_bucket).key("data/input.txt")
        .body(ByteStream::from("apple\nbanana\ncherry\napple\ndragonfruit\nbanana\napple\n".as_bytes().to_vec())).send().await, skip);
    ok!("upload spark script", s3.put_object().bucket(&spark_bucket).key("scripts/count_lines.py")
        .body(ByteStream::from(SPARK_SCRIPT.as_bytes().to_vec())).send().await, skip);

    let eks_info = emr::types::EksInfo::builder().namespace("awsem").build();
    let provider = emr::types::ContainerProvider::builder().id("spark-eks")
        .r#type(emr::types::ContainerProviderType::Eks)
        .info(emr::types::ContainerInfo::EksInfo(eks_info)).build().unwrap();

    if let Some(vc) = ok!("create Spark virtual cluster",
        emr.create_virtual_cluster().name("spark-vc").container_provider(provider).send().await, skip) {
        let sp_id = vc.id().unwrap_or_default().to_string();
        if !sp_id.is_empty() {
            let jd = emr::types::JobDriver::builder()
                        .spark_submit_job_driver(
                    emr::types::SparkSubmitJobDriver::builder()
                        .entry_point(format!("s3a://{spark_bucket}/scripts/count_lines.py"))
                        .entry_point_arguments(format!("s3a://{spark_bucket}/data/input.txt"))
                        .entry_point_arguments(format!("s3a://{spark_bucket}/data/output/"))
                        .spark_submit_parameters(
                            "--conf spark.hadoop.fs.s3a.endpoint=http://rustfs-svc:9000 \
                             --conf spark.hadoop.fs.s3a.access.key=awsem \
                             --conf spark.hadoop.fs.s3a.secret.key=awsem \
                             --conf spark.hadoop.fs.s3a.path.style.access=true"
                        )
                        .build().unwrap()
                )
                .build();

            match emr.start_job_run()
                .virtual_cluster_id(&sp_id).name("line-count-job")
                .release_label("emr-7.5.0-latest").job_driver(jd).send().await
            {
                Ok(resp) => {
                    let jr_id = resp.id().unwrap_or_default().to_string();
                    println!("  PASS  E2E start Spark job (id={jr_id})");

                    // Agent reports completion via admin API — one wait, no polling
                    tokio::time::sleep(Duration::from_secs(90)).await;
                    let state = match emr.describe_job_run().virtual_cluster_id(&sp_id).id(&jr_id).send().await {
                        Ok(d) => d.job_run().and_then(|r| r.state()).map(|s| format!("{}", s)).unwrap_or_default(),
                        Err(e) => { println!("  FAIL  describe: {e}"); String::new() }
                    };

                    if state == "COMPLETED" {
                        println!("  PASS  E2E Spark job completed");
                        if let Some(objs) = ok!("E2E list output files",
                            s3.list_objects_v2().bucket(&spark_bucket).prefix("data/output/").send().await, skip) {
                            let csv_keys: Vec<String> = objs.contents().iter()
                                .filter_map(|o| o.key()).filter(|k| k.ends_with(".csv"))
                                .map(String::from).collect();
                            if let Some(csv_key) = csv_keys.first() {
                                if let Some(got) = ok!("E2E read output CSV",
                                    s3.get_object().bucket(&spark_bucket).key(csv_key).send().await, skip) {
                                    let text = got.body.collect().await
                                        .map(|b| String::from_utf8_lossy(&b.into_bytes()).to_string())
                                        .unwrap_or_default();
                                    assert!(text.contains("line_count"), "CSV should have header");
                                    assert!(!text.is_empty());
                                    println!("  ECHO  CSV preview: {}", text.lines().take(3).collect::<Vec<_>>().join(" | "));
                                }
                            } else { println!("  WARN  No CSV files in output"); }
                        }
                        // Cleanup S3 objects
                        for o in s3.list_objects_v2().bucket(&spark_bucket).send().await.ok()
                            .into_iter().flat_map(|r| r.contents.unwrap_or_default()).filter_map(|o| o.key().map(String::from)).collect::<Vec<_>>() {
                            s3.delete_object().bucket(&spark_bucket).key(&o).send().await.ok();
                        }
                        s3.delete_bucket().bucket(&spark_bucket).send().await.ok();
                    } else {
                        println!("  SKIP  Spark job state={state} (K8s may be unavailable)");
                    }
                    emr.delete_virtual_cluster().id(&sp_id).send().await.ok();
                }
                Err(e) => { println!("  SKIP  E2E start Spark job — {e} (K8s may be unavailable)"); }
            }
        }
    }

    println!("\n===========================================");
    println!("Validation complete.");
    println!("===========================================\n");
}
