# skychat-web-backend

The API backendend for my web-based chat application (see skychat-web-frontend for the corresponding frontend)

By default, runs on `http://localhost:8000`.
This is only the database API, and not meant as a Website server.

Run with `cargo run` or
Build with `cargo build`

Ensure that a [mongodb instance](https://www.mongodb.com/docs/manual/administration/install-community/#std-label-install-mdb-community-edition) is running on the proper URL and port. (By default, `mongodb://localhost`)

NOTE: Your MongoDB needs to be running with a Replica Set so that this backend can track its changes via a Change Stream.

For example, if you installed MongoDB with Brew, simply add this to the file `/usr/local/etc/mongod.conf`:

```
replication:
  replSetName: "skyler_replica_set"
```
