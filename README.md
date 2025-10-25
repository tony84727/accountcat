Accountcat
==========

Personal finace helper.  
The project is still early stage. The development priority will focus on issues found when dogfooding to make it a useful daily driver for the author.

## Demo
Demo hosted on render.com free-tier: https://accountcat-demo.onrender.com/
⚠️Database will be purged every 30 days. You can take a look and try it out, but don't rely on it keeping your data.

## Generating mutual TLS certificates

The server binary now includes a helper for issuing self-signed PKI assets suitable for local mTLS testing. Run:

```
cargo run --bin accountcat -- pki --out-dir ./pki --server-san localhost --server-san 127.0.0.1
```

The command writes CA, server, and client certificates (alongside PKCS#12 bundles) into the target directory. Use `--pkcs12-password <value>` when you need password-protected archives, and pass additional `--client-san` or `--server-san` values to cover every hostname or IP your gRPC clients connect through.
