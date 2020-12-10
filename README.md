A HTTP-to-IC bridge
===================

**This is not an official DFINITY project, and is only a proof of concept by @nomeata**

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

To build for Amazon lambda, and hence musl, this is using a patched agent-rs without an openssl dependency for now.

Credits to @DavidM-D for a bunch of the ideas inside this.

[Candid]: https://github.com/dfinity/candid/blob/master/spec/Candid.md
[rust agent]: https://github.com/dfinity/agent-rs


