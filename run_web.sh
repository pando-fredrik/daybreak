set -e
cargo build --release --target wasm32-unknown-unknown
rm -rf generated && mkdir -p generated
wasm-bindgen target/wasm32-unknown-unknown/release/daybreak.wasm --out-dir generated --target web
cp daybreak.mp3 generated
cp index.html generated
(
  cd generated
  python -m http.server &
  pid=$!
  while ! nc -z localhost 8000; do
    sleep 0.1
  done
  open http://localhost:8000
  trap "kill $pid; exit" SIGINT
  wait $pid
)
