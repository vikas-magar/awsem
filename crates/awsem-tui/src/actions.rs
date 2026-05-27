use crate::aws_clients::AwsClients;

pub async fn create_bucket(aws: &AwsClients, name: &str) -> String {
    match aws.s3.create_bucket().bucket(name).send().await { Ok(_) => "Created".into(), Err(e) => format!("{e}") }
}

pub async fn delete_bucket(aws: &AwsClients, name: &str) -> String {
    match aws.s3.delete_bucket().bucket(name).send().await { Ok(_) => "Deleted".into(), Err(e) => format!("{e}") }
}

pub async fn create_user(aws: &AwsClients, username: &str, email: &str, password: &str) -> String {
    let mut req = aws.cognito.admin_create_user().user_pool_id("us-east-1_default").username(username);
    if !password.is_empty() { req = req.temporary_password(password); }
    if !email.is_empty() { req = req.user_attributes(aws_sdk_cognitoidentityprovider::types::AttributeType::builder().name("email").value(email).build().unwrap()); }
    match req.send().await { Ok(_) => "Created".into(), Err(e) => format!("{e}") }
}

pub async fn delete_user(aws: &AwsClients, username: &str) -> String {
    match aws.cognito.admin_delete_user().user_pool_id("us-east-1_default").username(username).send().await {
        Ok(_) => "Deleted".into(), Err(e) => format!("{e}"),
    }
}

pub async fn create_secret(aws: &AwsClients, name: &str, value: &str, description: &str) -> String {
    let mut req = aws.secrets.create_secret().name(name).secret_string(value);
    if !description.is_empty() { req = req.description(description); }
    match req.send().await { Ok(_) => "Created".into(), Err(e) => format!("{e}") }
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

pub async fn submit_job(aws: &AwsClients, vc: &str, entry_point: &str, role_arn: &str, release_label: &str, spark_params: &str) -> String {
    use aws_sdk_emrcontainers::types::{JobDriver, SparkSubmitJobDriver};
    let driver = JobDriver::builder()
        .spark_submit_job_driver(SparkSubmitJobDriver::builder().entry_point(entry_point).spark_submit_parameters(spark_params).build().expect("build"))
        .build();
    match aws.emr.start_job_run().name(format!("job-{}", chrono::Utc::now().timestamp()))
        .virtual_cluster_id(vc).execution_role_arn(role_arn)
        .release_label(release_label).job_driver(driver).send().await
    { Ok(_) => "Submitted".into(), Err(e) => format!("{e}") }
}
