mod test_utils;

use std::time::Duration;

use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task;

use async_h1::client;
use http_types::{headers, Method, Request, Response, StatusCode, Url};

const TEXT: &'static str = concat![
    "Chunk one\n",
    "data data\n",
    "\n",
    "Chunk two\n",
    "data data\n",
    "\n",
    "Chunk three\n",
    "data data\n",
];

#[async_std::test]
async fn async_h1_client() -> Result<(), http_types::Error> {
    let port = test_utils::find_port().await;
    let server = task::spawn(async move {
        let listener = async_std::net::TcpListener::bind(("127.0.0.1", port)).await?;

        let mut incoming = listener.incoming();
        let stream = incoming.next().await.unwrap().unwrap();

        async_h1::accept(stream, |_| async {
            let mut res = Response::new(StatusCode::Ok);
            res.set_body(TEXT.to_owned());
            Ok(res)
        })
        .await?;

        Ok(())
    });

    let client = task::spawn(async move {
        task::sleep(Duration::from_millis(100)).await;

        let stream = TcpStream::connect(("localhost", port)).await?;
        let peer_addr = stream.peer_addr()?;
        let url = Url::parse(&format!("http://{}", peer_addr))?;
        let req = Request::new(Method::Get, url);
        let mut res = client::connect(stream.clone(), req).await?;

        assert_eq!(res[headers::CONTENT_LENGTH], TEXT.len().to_string());
        let mut bytes = Vec::with_capacity(1024);
        res.read_to_end(&mut bytes).await?;
        assert_eq!(bytes.as_slice(), TEXT.as_bytes());

        Result::<(), http_types::Error>::Ok(())
    });

    server.race(client).await
}
