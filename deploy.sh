#!/usr/bin/env bash
set -e

cargo build --features with-lambda --release --target x86_64-unknown-linux-musl
cp ./target/x86_64-unknown-linux-musl/release/ic-http-lambda ./bootstrap
zip lambda.zip bootstrap
rm bootstrap
aws lambda update-function-code --region eu-central-1 --function-name ic-http-lambda --zip-file fileb://./lambda.zip
