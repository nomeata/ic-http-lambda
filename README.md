A HTTP-to-IC bridge
===================

**This is not an official DFINITY project, and is only a proof of concept by
@nomeata**

This repository contains code necessary to talk to DFINITY Canisters directly
via HTTP. It expects HTTP requests at `https://<canister_id>.ic.nomeata.de/`,
packs the HTTP request data into a [Candid] value, and uses the official [rust
agent] to talk to the given canister, which is expected to implement the
interface listed below. It first tries a query call, and if the query call
indicates that state changes need to be done, upgrades to an update call. The
call replies with a HTTP response in a Candid value, which this code sends
back.

The present code ran run as a local websever (for local development), or as an
Amazon lambda function.

The interface expected from canisters is
```
type request = record {
  method : text;
  headers : vec record {blob; blob};
  uri : text;
  body : blob;
};
type response = record {
  status : nat16;
  headers : vec record {blob; blob};
  body : blob;
  upgrade : bool;
};
service : {
  http_query : (request) -> (response);
  http_update : (request) -> (response);
}
```

# Deploying as an Amazon Lambda function
## Create the Lambda function

From the [Lambda service](https://aws.amazon.com/lambda) of your AWS Management
Console, create an AWS Lambda project. Choose "Provide your own bootstrap on
Amazon Linux 2" for the runtime.

### Configure your Lambda function

In the Configuration tab, click the Edit button for "General configuration" and
increase the timeout to 30 seconds.

## Upload the code

Make sure `main.rs` uses a domain you own and not nomeata.de.

Make sure `deploy.sh` specifies the correct region and your new lambda function
name.

Run `deploy.sh`.

<sub> note: Make note of the additional steps
[here](https://aws.amazon.com/blogs/opensource/rust-runtime-for-aws-lambda/) if
you are compiling on Mac OS.

## Create the HTTP API Gateway

From the [API Gateway service](https://aws.amazon.com/api-gateway/) of your AWS
Management Console, create an HTTP API.

Add a Lambda integration and select your new function.

Add an "ANY" Route with `/{proxy+}` as the Resource path and your lambda as the
Integration target.

Leave `$default` as the Stage name.

### Configure your gateway

Choose "Custom domain names" from the menu. Create one for your wildcard ic
domain (e.g. `*.ic.yourdomain.com`) and create an ACM Certificate for it. Request
a public certificate with two domain names: `*.ic.yourdomain.com` and
`ic.yourdomain.com`. Complete validation by email or DNS.

Once Amazon has had a chance to verify ownership of your domain, refresh the
"Create Domain Name" page and select the new certificate.

In the API Mappings section for the custom domain name, add an API mapping from
the new gateway on the $default stage and leave "Path" empty.

Under "Develop" in the menu on the left, chose "Integrations". Select "ANY"
under `/{proxy+}` and click "Manage Integration". Edit the "Integration
details" and under "Advanced settings", make sure the timeout is 30000
milliseconds and set the "Payload version format" to 1.0.

Before leaving the main gateway configuration page, make note of the Invoke
URL.

## Configure your DNS

In the DNS settings for your domain, add two new CNAME records, one for `ic`
and one for `*.ic`, both pointing at your Invoke URL (minus the https://).

At this point, navigating to https://\<cid\>.ic.yourdomain.com should properly
forward your request to and return the response from your caniser.

Credits to @DavidM-D for a bunch of the ideas inside this.

[Candid]: https://github.com/dfinity/candid/blob/master/spec/Candid.md
[rust agent]: https://github.com/dfinity/agent-rs
