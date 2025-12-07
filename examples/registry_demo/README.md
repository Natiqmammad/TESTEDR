# Registry Demo

This walkthrough shows how to exercise the Phase 1 package registry using the local `nightscript-server` and the `apexrc` CLI.

## 1. Start the registry

```bash
cargo run --manifest-path nightscript-server/Cargo.toml
```

The server listens on `127.0.0.1:5665`.

## 2. Create a user and login

```bash
curl -X POST http://127.0.0.1:5665/api/v1/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"demo","email":"demo@example.com","password":"secret"}'

printf "demo\nsecret\n" | apexrc login --registry http://127.0.0.1:5665
apexrc whoami
```

## 3. Publish a library

```bash
mkdir -p hello-afml/src
cat <<'TOML' > hello-afml/Apex.toml
[package]
name = "hello-afml"
version = "0.1.0"
language = "afml"
description = "Registry example"
[dependencies]
[registry]
url = "http://127.0.0.1:5665"
TOML

cat <<'AFML' > hello-afml/src/lib.afml
import forge.log as log;
fun greet() { log.info("hello from registry"); }
AFML

cd hello-afml
apexrc publish
```

## 4. Consume the package

```bash
mkdir -p ../registry-app/src
cat <<'TOML' > ../registry-app/Apex.toml
[package]
name = "registry-app"
version = "0.1.0"
language = "afml"
[dependencies]
hello-afml = "^0.1.0"
[registry]
url = "http://127.0.0.1:5665"
TOML

cat <<'AFML' > ../registry-app/src/main.afml
import hello-afml as hello;
fun apex() { hello.greet(); }
AFML

cd ../registry-app
apexrc add hello-afml
apexrc install
apexrc run
```

`apexrc run` prints `hello from registry`, proving the end-to-end flow.
