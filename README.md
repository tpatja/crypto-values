## crypto-values

Simple system for tracking value of crypto holdings written in Rust.

Uses a Google Sheets document with a worksheet for holdings and another worksheet
for daily total value. A new row will be added to the daily total value worksheet
if the current date is not equal to last row's date and the price for current date
is updated every time it increases, making it track the daily high for total value.

Produces two executables:
- a CLI that can be used for querying values for holdings and updating total value in
  the google sheet
- an AWS lambda executable that is used for updating total value in the google sheet

Environment variables:
- `GOOGLE_SERVICE_ACCOUNT_JSON` Service account JSON content for accessing the Google Sheet.
- `GOOGLE_SHEET_ID` Google sheet ID.
- `CMC_API_KEY` Coinmarketcap.com API key. If you run the AWS lambda every 5min, you stay
 within the free plan limits.

Makefile targets:
- `build-aws-lambda`: build x86-64 executable to be deployed to AWS lambda
- `deploy-aws-lambda`: create all needed AWS resources and deploy lambda binary. Resources include:
  - IAM role
  - IAM role policy
  - lambda function
  - events rule for scheduling function to run function every 5min
- `undeploy-aws-lambda`: destroy all AWS resources created by `deploy-aws-lambda`
- `build-cli`: build CLI executable


CLI usage

```
$ crypto-values
Gets crypto values from coinmarketcap API and uses a google sheet
for managing holdings and tracking daily total value.

Usage: crypto-values [OPTIONS]

Options:
      --shell-completions <GENERATOR>  Generates shell completions for the given shell [possible values: bash, elvish, fish, powershell, zsh]
  -u, --update-gsheet-total-value      Updates daily total value in google sheet if current value is higher than existing value or row for current date does not exist
  -s, --show-prices                    Lists all holdings with current prices and total value
  -h, --help                           Print help information
  -V, --version                        Print version information
```
Example output:

```
$ crypto-values -s
BTC-EUR: 0.0100         * 15865.71      Total: 158.66   (57%)
ETH-EUR: 0.1000         * 1184.63       Total: 118.46   (42%)
Total value (EUR): 277.12
$
```

TODO:
- make fiat currency configurable (currently always uses EUR)
