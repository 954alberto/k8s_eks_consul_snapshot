use anyhow::{Context, Result};
use chrono::Local;
use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use curl::easy::{Easy, List};
use rusoto_core::{HttpClient, Region};
use rusoto_credential::StaticProvider;
use rusoto_s3::{PutObjectRequest, S3Client, S3};
use rusoto_signature::stream::ByteStream;
use rusoto_sts::{AssumeRoleWithWebIdentityRequest, Sts, StsClient};
use std::{env, fs};

#[tokio::main]
async fn main() -> Result<()> {
    let m = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("aws_role_arn")
                .long("aws_role_arn")
                .env("AWS_ROLE_ARN")
                .help("The arn of the AWS IAM role")
                .required(true),
        )
        .arg(
            Arg::with_name("aws_web_identity_token_file")
                .long("aws_web_identity_token_file")
                .env("AWS_WEB_IDENTITY_TOKEN_FILE")
                .help("The file containing the AWS token")
                .required(true),
        )
        .arg(
            Arg::with_name("s3_bucket_name")
                .long("s3_bucket_name")
                .env("S3_BUCKET_NAME")
                .help("The S3 bucket to upload to")
                .required(true),
        )
        .arg(
            Arg::with_name("consul_http_addr")
                .long("consul_http_addr")
                .env("CONSUL_HTTP_ADDR")
                .help("The consul endpoint including the path /v1/snapshot")
                .required(true),
        )
        .arg(
            Arg::with_name("consul_http_token")
                .long("consul_http_token")
                .env("CONSUL_HTTP_TOKEN")
                .help("The consul ACL token to access")
                .required(true),
        )
        .get_matches();

    let aws_web_identity_token =
        fs::read_to_string(m.value_of("aws_web_identity_token_file").unwrap())
            .context("Error reading the AWS identity token file")?;

    let http_header = format!(
        "X-Consul-Token: {}",
        m.value_of("consul_http_token").unwrap()
    );
    let sts = StsClient::new(Region::EuWest1);

    let result = sts
        .assume_role_with_web_identity(AssumeRoleWithWebIdentityRequest {
            duration_seconds: Some(900),
            role_arn: m.value_of("aws_role_arn").unwrap().to_string(),
            role_session_name: "dev".to_string(),
            web_identity_token: aws_web_identity_token,
            ..Default::default()
        })
        .await
        .context("Failed to get AWS credentials")?;

    let creds = result.credentials.context("No credentials available")?;

    let mut list = List::new();
    list.append(&http_header).unwrap();

    let timestamp = Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
    let file_name = format!("snapshot_{}.tar.gz", timestamp);

    let mut data = Vec::new();
    let mut handle = Easy::new();
    handle.url(m.value_of("consul_http_addr").unwrap()).unwrap();
    handle.http_headers(list).unwrap();
    handle.perform().context("Could not send request")?;
    {
        let mut transfer = handle.transfer();
        transfer
            .write_function(|new_data| {
                data.extend_from_slice(new_data);
                Ok(new_data.len())
            })
            .unwrap();
        transfer.perform().unwrap();
    }
    let mut object_req = PutObjectRequest::default();
    object_req.bucket = m.value_of("s3_bucket_name").unwrap().to_string();
    object_req.key = file_name.to_owned();
    object_req.content_type = Some("application/x-compressed-tar".to_owned());
    object_req.body = Some(ByteStream::from(data));

    let dispatcher = HttpClient::new().unwrap();

    let provider = StaticProvider::new(
        creds.access_key_id,
        creds.secret_access_key,
        Some(creds.session_token),
        None,
    );

    let client = S3Client::new_with(dispatcher, provider, Region::EuWest1);

    let result = client
        .put_object(object_req)
        .await
        .expect("\nCouldn't put object\n");
    println!("result is {:#?}", result);

    Ok(())
}
