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
                overflow:hidden;padding:10px 5px;word-break:normal;}
            .tg th{border-color:white;border-style:solid;border-width:1px;font-family:Arial, sans-serif;font-size:14px;
                font-weight:normal;overflow:hidden;padding:10px 5px;word-break:normal;}
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
            <a href="/config">Configuration</a>
            <br>
            <a href="https://github.com/vinteumorg/pleblottery">Source Code</a>
            <br><br>
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
                overflow:hidden;padding:10px 5px;word-break:normal;}
            .tg th{border-color:white;border-style:solid;border-width:1px;font-family:Arial, sans-serif;font-size:14px;
                font-weight:normal;overflow:hidden;padding:10px 5px;word-break:normal;}
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

// Define the router for HTML routes
pub fn html_routes() -> Router {
    Router::new()
        .route("/", axum::routing::get(serve_index))
        .route("/config", axum::routing::get(serve_config_html))
}
