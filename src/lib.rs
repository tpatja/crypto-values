use google_sheets4 as sheets4;
use std::collections::HashMap;
use std::env;
use std::process;

use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use sheets4::api::ValueRange;
use sheets4::oauth2::ServiceAccountKey;
use sheets4::{hyper, hyper_rustls, oauth2, Sheets};

async fn cmc_json_to_price_map(
    json: &serde_json::Value,
    symbols: Vec<&str>,
) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
    let mut prices: HashMap<String, f64> = HashMap::new();
    let obj = json.as_object().expect("JSON format error");
    symbols.iter().for_each(|s| {
        let price = obj["data"][s]["quote"]["EUR"]["price"].as_f64().unwrap();
        prices.insert(s.to_string(), price);
    });
    Ok(prices)
}

pub async fn get_cmc_eur_prices(
    symbols: Vec<&str>,
) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
    let api_key = env::var("CMC_API_KEY").unwrap_or_else(|_| {
        eprintln!("CMC_API_KEY must be set.");
        process::exit(1);
    });

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("X-CMC_PRO_API_KEY", api_key.parse().unwrap());
    let symbols_str = symbols.join(",");
    let res = reqwest::Client::new()
        .get(format!(
            "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol={}&convert=EUR",
            symbols_str
        ))
        .headers(headers)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    let res = cmc_json_to_price_map(&res, symbols).await?;
    Ok(res)
}

async fn get_google_sheet_client(
) -> Result<Sheets<HttpsConnector<HttpConnector>>, Box<dyn std::error::Error>> {
    let service_account_json = env::var("GOOGLE_SERVICE_ACCOUNT_JSON").unwrap_or_else(|_| {
        eprintln!("GOOGLE_SERVICE_ACCOUNT_JSON must be set.");
        process::exit(1);
    });
    let service_account_key: ServiceAccountKey =
        sheets4::oauth2::parse_service_account_key(&service_account_json)?;
    let client = hyper::Client::builder().build(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_only()
            .enable_http1()
            .enable_http2()
            .build(),
    );

    let auth =
        oauth2::ServiceAccountAuthenticator::with_client(service_account_key, client.clone())
            .build()
            .await?;
    let client = Sheets::new(client, auth);
    Ok(client)
}

pub async fn get_holdings_from_google_sheet(
    client: &Sheets<HttpsConnector<HttpConnector>>,
    gsheet_id: &str,
) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
    let res = client
        .spreadsheets()
        .values_get(&gsheet_id, "Holdings!A:I")
        .doit()
        .await?
        .1
        .values;
    let res: Vec<(String, f64)> = res.unwrap()[1..]
        .iter()
        .map(|row| {
            let price: f64 = row[row.len() - 1].replace(",", "").parse().unwrap_or(0.0);
            (row[0].clone(), price)
        })
        .collect();
    let res = HashMap::from_iter(res);
    Ok(res)
}

fn get_symbol_price(cmc_prices: &HashMap<String, f64>, symbol: &str) -> f64 {
    let price = if symbol == "EUR" {
        &1.0
    } else {
        cmc_prices.get(symbol).unwrap_or(&0.0)
    };
    *price
}

pub async fn get_updated_values(
    print_values: bool,
    update_gsheet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let gsheet_id = env::var("GSHEET_ID").unwrap_or_else(|_| {
        eprintln!("GSHEET_ID must be set.");
        process::exit(1);
    });
    let client = get_google_sheet_client().await?;
    let holdings = get_holdings_from_google_sheet(&client, &gsheet_id).await?;
    let symbols = holdings
        .keys()
        .filter(|s| *s != "EUR")
        .map(|s| s.as_str())
        .collect();
    let cmc_prices = get_cmc_eur_prices(symbols).await?;
    let total = holdings.iter().fold(0.0, |acc, (k, v)| {
        let price = get_symbol_price(&cmc_prices, k);
        acc + (v * price)
    });
    if print_values {
        let mut holdings_vec: Vec<(&String, &f64)> = holdings.iter().collect();
        holdings_vec.sort_by(|a, b| a.0.cmp(b.0));
        for (symbol, amount) in holdings_vec {
            let price = get_symbol_price(&cmc_prices, symbol);
            let value = amount * price;
            let pct: i8 = ((value / total) * 100.0) as i8;
            println!(
                "{}-EUR: {:.4}  \t* {:.2}  \tTotal: {:.2} \t({}%)",
                symbol, amount, price, value, pct
            );
        }
        println!("Total value (EUR): {:.2}", total);
    }
    if update_gsheet {
        update_gsheet_total_value(&client, &gsheet_id, total).await?;
    }

    Ok(())
}

async fn update_gsheet_total_value(
    client: &Sheets<HttpsConnector<HttpConnector>>,
    gsheet_id: &str,
    total_value: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = client
        .spreadsheets()
        .values_get(&gsheet_id, "Value!A:B")
        .doit()
        .await?
        .1
        .values
        .unwrap();
    let last_row_num = res.len() - 1;
    let latest_date_str: &str = &res[last_row_num][0];
    let latest_total_value: f64 = res[last_row_num][1]
        .replace(",", "")
        .parse()
        .expect("latest total value is NaN");

    let todays_date_str = chrono::Local::now().format("%-d.%-m.%Y").to_string();

    if latest_total_value < total_value || latest_date_str != todays_date_str.as_str() {
        if latest_total_value < total_value {
            eprintln!(
                "Total value increased from {} to {}, updating google sheet",
                latest_total_value, total_value
            );
        } else if latest_date_str != todays_date_str.as_str() {
            eprintln!(
                "Date changed from {} to {}, updating google sheet",
                latest_date_str, todays_date_str
            );
        }

        let mut req = ValueRange::default();
        let new_total_value = format!("{}", total_value);
        let vals: Vec<Vec<String>> = vec![vec![todays_date_str.clone(), new_total_value]];
        req.values = Some(vals);
        let row_num = if latest_date_str == todays_date_str.as_str() {
            last_row_num + 1
        } else {
            last_row_num + 2
        };
        let range = format!("Value!A{}:B{}", row_num, row_num);
        let _ = client
            .spreadsheets()
            .values_update(req, &gsheet_id, &range)
            .value_input_option("USER_ENTERED")
            .doit()
            .await?;
    }
    Ok(())
}
