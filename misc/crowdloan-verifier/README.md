# Crowdloan Reward List verifier

This tool provides a way to verify a csv file list of contributors with hardcoded map of rewards inside the pallet

For usage details, `cargo run -- --help`

1. For a checking the whole file: ` cargo run -- -p <path to csv file>`
2. Search the details of a specific address: ` cargo run -- -p <path to csv file> -u <user address>`