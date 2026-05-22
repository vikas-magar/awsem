use crate::aws_clients::AwsClients;
use aws_sdk_s3::primitives::ByteStream;

pub async fn create_bucket(aws: &AwsClients, name: &str) -> String {
    match aws.s3.create_bucket().bucket(name).send().await { Ok(_) => "Created".into(), Err(e) => format!("{e}") }
}

pub async fn delete_bucket(aws: &AwsClients, name: &str) -> String {
    match aws.s3.delete_bucket().bucket(name).send().await { Ok(_) => "Deleted".into(), Err(e) => format!("{e}") }
}

pub async fn upload_object(aws: &AwsClients, bucket: &str, key: &str, content: &[u8]) -> String {
    match aws.s3.put_object().bucket(bucket).key(key).body(ByteStream::from(content.to_vec())).send().await {
        Ok(_) => "Uploaded".into(), Err(e) => format!("{e}"),
    }
}

pub async fn delete_object(aws: &AwsClients, bucket: &str, key: &str) -> String {
    match aws.s3.delete_object().bucket(bucket).key(key).send().await { Ok(_) => "Deleted".into(), Err(e) => format!("{e}") }
}

pub async fn create_user(aws: &AwsClients, username: &str, password: &str) -> String {
    match aws.cognito.admin_create_user().user_pool_id("us-east-1_default").username(username).temporary_password(password).send().await {
        Ok(_) => "User created".into(), Err(e) => format!("{e}"),
    }
}

pub async fn delete_user(aws: &AwsClients, username: &str) -> String {
    match aws.cognito.admin_delete_user().user_pool_id("us-east-1_default").username(username).send().await {
        Ok(_) => "Deleted".into(), Err(e) => format!("{e}"),
    }
}

pub async fn create_secret(aws: &AwsClients, name: &str, value: &str) -> String {
    match aws.secrets.create_secret().name(name).secret_string(value).send().await {
        Ok(_) => "Created".into(), Err(e) => format!("{e}"),
    }
}

pub async fn edit_secret(aws: &AwsClients, name: &str, value: &str) -> String {
    match aws.secrets.put_secret_value().secret_id(name).secret_string(value).send().await {
        Ok(_) => "Updated".into(), Err(e) => format!("{e}"),
    }
}

pub async fn delete_secret(aws: &AwsClients, name: &str) -> String {
    match aws.secrets.delete_secret().secret_id(name).force_delete_without_recovery(true).send().await {
        Ok(_) => "Deleted".into(), Err(e) => format!("{e}"),
    }
}

pub async fn delete_function(aws: &AwsClients, name: &str) -> String {
    match aws.lambda.delete_function().function_name(name).send().await { Ok(_) => "Deleted".into(), Err(e) => format!("{e}") }
}

pub async fn invoke_lambda(aws: &AwsClients, func: &str, payload: &str) -> String {
    let blob = aws_sdk_lambda::primitives::Blob::new(payload.as_bytes().to_vec());
    match aws.lambda.invoke().function_name(func).payload(blob).send().await {
        Ok(r) => r.payload().map(|b| String::from_utf8_lossy(b.as_ref()).to_string()).unwrap_or_default(),
        Err(e) => format!("{e}"),
    }
}

pub async fn delete_vc(aws: &AwsClients, id: &str) -> String {
    match aws.emr.delete_virtual_cluster().id(id).send().await { Ok(_) => "Deleted".into(), Err(e) => format!("{e}") }
}

pub async fn cancel_job(aws: &AwsClients, vc: &str, job: &str) -> String {
    match aws.emr.cancel_job_run().virtual_cluster_id(vc).id(job).send().await { Ok(_) => "Cancelled".into(), Err(e) => format!("{e}") }
}

pub async fn submit_job(aws: &AwsClients, vc: &str, entry_point: &str) -> String {
    let jd = aws_sdk_emrcontainers::types::JobDriver::builder()
        .spark_submit_job_driver(
            aws_sdk_emrcontainers::types::SparkSubmitJobDriver::builder()
                .entry_point(entry_point)
                .spark_submit_parameters("--conf spark.executor.instances=1")
                .build()
                .expect("SparkSubmitJobDriver build")
        ).build();
    match aws.emr.start_job_run().name(format!("job-{}", chrono::Utc::now().timestamp()))
        .virtual_cluster_id(vc)
        .execution_role_arn("arn:aws:iam::123456789012:role/emr-role")
        .release_label("emr-7.5.0-latest")
        .job_driver(jd).send().await
    {
        Ok(_) => "Job submitted".into(), Err(e) => format!("{e}"),
    }
}
