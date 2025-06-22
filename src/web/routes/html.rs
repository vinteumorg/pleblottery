use axum::{response::Html, Router};

// Serve the HTML page for /
pub async fn serve_index() -> Html<&'static str> {
    Html(
        r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>pleblottery</title>
        <style type="text/css">
            .tg {border-collapse:collapse;border-spacing:0;}
            .tg td{border-color:white;border-style:solid;border-width:1px;font-family:Arial, sans-serif;font-size:14px;
                overflow:hidden;padding:10px 5px;word-break:normal;text-align:center;}
            .tg th{border-color:white;border-style:solid;border-width:1px;font-family:Arial, sans-serif;font-size:14px;
                font-weight:normal;overflow:hidden;padding:10px 5px;word-break:normal;text-align:center;}
            .tb {}
            .tb td{border-width: 0}
            body {background-color:#051426;color:white;}
            a {color:white}
        </style>
    </head>
    <body>
        <center>
            <div style="background-color:#051426;color:white;"> 
                <br>
                <b><span style="color: #3CAD65">$</span> pleblottery <span style="color: #D6AF46">#</span></b>
                <br><br>
                <img src="/static/images/pleblottery.png" alt="pleblottery logo">
                <br><br>
                a Rust-based hashrate aggregator for a pleb-friendly and fully sovereign solo/lottery Bitcoin mining experience over <a href="https://stratumprotocol.org">Stratum V2</a>
                <br><br>
            </div>
            <br>
            <a href="/">Home</a>
            <br>
            <a href="/dashboard">Dashboard</a>
            <br>
            <a href="/config">Configuration</a>
            <br>
            <a href="https://github.com/vinteumorg/pleblottery">Source Code</a>
            <br>
            <hr>
            <br>
            ⛏️ plebs be hashin ⚡
            <br><br>
        </center>
    </body>
    </html>
    "#,
    )
}

// Serve the HTML page for /config
pub async fn serve_config_html() -> Html<&'static str> {
    Html(
        r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>pleblottery - Configuration</title>
        <style type="text/css">
            .tg {border-collapse:collapse;border-spacing:0;}
            .tg td{border-color:white;border-style:solid;border-width:1px;font-family:Arial, sans-serif;font-size:14px;
                overflow:hidden;padding:10px 5px;word-break:normal;text-align:center;}
            .tg th{border-color:white;border-style:solid;border-width:1px;font-family:Arial, sans-serif;font-size:14px;
                font-weight:normal;overflow:hidden;padding:10px 5px;word-break:normal;text-align:center;}
            .tb {}
            .tb td{border-width: 0}
            body {background-color:#051426;color:white;}
            a {color:white}
        </style>
        <script src="https://unpkg.com/htmx.org"></script>
    </head>
    <body>
        <center>
            <div style="background-color:#051426;color:white;"> 
                <br>
                <b><span style="color: #3CAD65">$</span> pleblottery <span style="color: #D6AF46">#</span></b>
                <br><br>
            </div>
            <br>
            <a href="/">Home</a>
            <br><br>
            <hr>
            <br>
            <div id="config-container" class="mt-4">
                <table class="tg">
                    <thead>
                        <tr>
                            <th>Configuration Parameter</th>
                            <th>Value</th>
                            <th>Description</th>
                        </tr>
                    </thead>
                    <tbody hx-get="/api/config" hx-trigger="load" hx-target="this">
                        <!-- Rows will be dynamically loaded here -->
                    </tbody>
                </table>
                <br>
                <b>Note:</b> this page simply displays the configuration parameters that were loaded from the config file.
                <br>
                To change the configuration, edit <code>config.toml</code> and restart the service.
                <br>
            </div>
            <br>
            <hr>
            <br>
             ⛏️ plebs be hashin ⚡
            <br><br>
        </center>
    </body>
    </html>
    "#,
    )
}

pub async fn serve_dashboard_html() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>pleblottery - Dashboard</title>
    <style type="text/css">
        .tg {
            border-collapse: collapse;
            border-spacing: 0;
            width: 100%;
        }

        .tg td,
        .tg th {
            border-color: white;
            border-style: solid;
            border-width: 1px;
            font-family: Arial, sans-serif;
            font-size: 14px;
            overflow: hidden;
            padding: 10px 5px;
            word-break: normal;
            text-align: center;
            width: 50%;
        }

        /* Ensure equal width for all cells */
        .tg th {
            font-weight: bold;
            text-align: center;
            /* Center-align the table headers */
        }

        .tb {}

        .tb td {
            border-width: 0
        }

        body {
            background-color: #051426;
            color: white;
            margin: 0;
            padding: 0;
        }

        a {
            color: white;
            text-decoration: none;
        }

        .container {
            margin: 0 auto;
            padding: 20px;
        }

        .responsive-table {
            overflow-x: auto;
        }

        .table-container {
            display: flex;
            justify-content: center;
            gap: 20px;
            flex-wrap: wrap;
        }

        .table-container .responsive-table {
            flex: 1;
            max-width: 900px;
            min-width: 600px;
        }

        #clients-container {
            display: flex;
            flex-wrap: wrap;
            justify-content: center;
            gap: 20px;
        }

        #clients-container > div {
            flex: 0 1 auto;
            min-width: 300px;
            max-width: 400px;
        }

        #clients-container .tg {
            width: 100%;
        }

        .mining-stats-container {
            max-width: 400px;
            min-width: 300px;
            margin: 0 auto;
        }

        .tg tr {
            height: 50px;
        }

        /* Ensure all rows have the same height */
        @media (max-width: 768px) {

            .tg td,
            .tg th {
                font-size: 12px;
                padding: 8px;
            }

            .tg th {
                font-weight: normal;
            }
        }
    </style>
    <script src="https://unpkg.com/htmx.org"></script>
</head>

<body>
    <center>
        <div class="container" style="background-color:#051426;color:white;">
            <br>
            <b><span style="color: #3CAD65">$</span> pleblottery <span style="color: #D6AF46">#</span></b>
            <br>
        </div>
        <a href="/">Home</a>
        <br>
        <hr>
        <div class="table-container">
            <div id="block-height-container" class="responsive-table">
                <table class="tg">
                    <thead>
                        <tr>
                            <th colspan="2">Chain Tip</th>
                        </tr>
                    </thead>
                    <tbody hx-get="/api/latest-prev-hash" hx-trigger="every 2s" hx-target="this" hx-swap="innerHTML">
                        <tr>
                            <td>Height</td>
                            <td>Loading...</td>
                        </tr>
                        <tr>
                            <td>Prev Hash</td>
                            <td>Loading...</td>
                        </tr>
                        <tr>
                            <td>nBits</td>
                            <td>Loading...</td>
                        </tr>
                        <tr>
                            <td>Target</td>
                            <td>Loading...</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div id="dashboard-container" class="responsive-table">
                <table class="tg">
                    <thead>
                        <tr>
                            <th colspan="2">Latest Template</th>
                        </tr>
                    </thead>
                    <tbody hx-get="/api/latest-template" hx-trigger="every 2s" hx-target="this" hx-swap="innerHTML">
                        <tr>
                            <td>Template ID</td>
                            <td>Loading...</td>
                        </tr>
                        <tr>
                            <td>Version</td>
                            <td>Loading...</td>
                        </tr>
                        <tr>
                            <td>Coinbase Value</td>
                            <td>Loading...</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
        <br><br>
        <div id="dashboard-container" class="responsive-table mining-stats-container">
            <table class="tg">
                <thead>
                    <tr>
                        <th colspan="2">Mining Stats</th>
                    </tr>
                </thead>
                <tbody hx-get="/api/mining-stats" hx-trigger="every 2s" hx-target="this" hx-swap="innerHTML">
                    <tr>
                        <td>Total Clients</td>
                        <td>Loading ...</td>
                    </tr>
                    <tr>
                        <td>Total Shares</td>
                        <td>Loading ...</td>
                    </tr>
                    <tr>
                        <td>Best Share</td>
                        <td>Loading ...</td>
                    </tr>
                    <tr>
                        <td>Total Hashrate</td>
                        <td>Loading ...</td>
                    </tr>
                    <tr>
                        <td>💰 Blocks Found 💰</td>
                        <td>Loading ...</td>
                    </tr>
                </tbody>
            </table>
        </div>
        <br><br>
        <div id="clients-container" hx-get="/api/clients" hx-trigger="every 2s" hx-target="this" hx-swap="innerHTML">
            <!-- Client tables will be dynamically loaded here -->
        </div>
        <hr>
        ⛏️ plebs be hashin ⚡
        <br>
    </center>
</body>

</html>
    "#,
    )
}

pub async fn serve_clients_html() -> Html<&'static str> {
    Html(
        r#"
        <!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>pleblottery - Clients</title>
    <style type="text/css">
        .tg {
            border-collapse: collapse;
            border-spacing: 0;
        }

        .tg td,
        .tg th {
            border-color: white;
            border-style: solid;
            border-width: 1px;
            font-family: Arial, sans-serif;
            font-size: 14px;
            overflow: hidden;
            padding: 10px 5px;
            word-break: normal;
            text-align: center;
            width: 50%;
        }

        /* Ensure equal width for all cells */
        .tg th {
            font-weight: bold;
            text-align: center;
            /* Center-align the table headers */
        }

        .tb td {
            border-width: 0
        }

        body {
            background-color: #051426;
            color: white;
            margin: 0;
            padding: 0;
        }

        a {
            color: white;
            text-decoration: none;
        }

        .container {
            display: flex;
            flex-wrap: wrap;
            padding: 20px;
            overflow-x: auto;
            justify-content: space-evenly;
            align-items: center;
        }

        .tg tr {
            height: 50px;
        }

        /* Ensure all rows have the same height */
        @media (max-width: 768px) {

            .tg td,
            .tg th {
                font-size: 12px;
                padding: 8px;
            }

            .tg th {
                font-weight: normal;
            }
        }
    </style>
    <script src="https://unpkg.com/htmx.org"></script>
</head>

<body>
    <center>
        <div class="container" style="background-color:#051426;color:white;">
            <br>
            <b><span style="color: #3CAD65">$</span> pleblottery <span style="color: #D6AF46">#</span></b>
            <br>
        </div>
        <a href="/">Home</a>
        <br>
        <hr>
        <div class="container" hx-get="/api/clients" hx-trigger="every 2s" hx-target="this" hx-swap="innerHTML">
            <div>
            <h2>Nothing here yet</h2>
            </div>
        </div>
        <hr>
        ⛏️ plebs be hashin ⚡
        <br>
    </center>
</body>

</html>
    "#,
    )
}

// Define the router for HTML routes
pub fn html_routes() -> Router {
    Router::new()
        .route("/", axum::routing::get(serve_index))
        .route("/config", axum::routing::get(serve_config_html))
        .route("/dashboard", axum::routing::get(serve_dashboard_html))
}
