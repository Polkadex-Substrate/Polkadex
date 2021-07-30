set -e

mkdir -p artifacts

for dir in "$@"
do
    cargo +nightly contract build --manifest-path "contracts/$dir/Cargo.toml" || exit 1
done

find target/ink/ -maxdepth 1 -mindepth 1 -type d | while read dir; do
  \cp -rf "${dir}"/*.contract artifacts/
  dirName=$(echo "${dir}" | cut -d/ -f3)
  cp "${dir}"/metadata.json artifacts/"${dirName}".metadata.json
  \cp "${dir}"/*.wasm artifacts/
done
