use {
    dango_app::LatestVaaResponse, futures_util::StreamExt, grug::JsonDeExt, pyth::PythClient,
    reqwest::Client, std::time::Duration, tokio::time::sleep,
};

#[tokio::test]
async fn test() {
    let client = Client::new();

    // URL del server SSE
    let url = "https://hermes.pyth.network/v2/updates/price/stream";

    // Effettua la richiesta GET e ottiene la risposta come stream
    let mut response = client
        .get(url)
        .query(&[(
            "ids[]",
            "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
        )])
        .query(&[(
            "ids[]",
            "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
        )])
        .query(&[("parsed", "false")])
        .query(&[("encoding", "base64")])
        .send()
        .await
        .unwrap()
        .bytes_stream();

    println!("Connesso al server SSE!");

    // Legge gli eventi in streaming
    let mut times = 0;
    // while let Some(chunk) = streaming.next().await {
    //     match chunk {
    //         Ok(bytes) => {
    //             let a1 = String::from_utf8(bytes.to_vec()).unwrap();
    //             // let a2 = &a1.trim()[5..];
    //             println!("messaggio: {}", a1);
    //             // println!("{}", a2);
    //             // let res = a2
    //             //     .as_bytes()
    //             //     .deserialize_json::<LatestVaaResponse>()
    //             //     .unwrap();
    //             // println!("Messaggio ricevuto");
    //             // println!("{:?}", res);
    //             // println!();
    //             times += 1;
    //         },
    //         Err(e) => eprintln!("Errore nella ricezione: {}", e),
    //     }
    //     if times > 10 {
    //         break;
    //     }
    // }

    let mut buffer = Vec::new(); // Buffer per ricostruire gli eventi
    while let Some(chunk) = response.next().await {
        if times > 5 {
            break;
        }
        match chunk {
            Ok(bytes) => {
                buffer.extend_from_slice(&bytes); // Aggiungi il chunk al buffer

                // Prova a trovare un delimitatore di evento (\n\n) nel buffer
                while let Some(pos) = find_event_delimiter(&buffer) {
                    let mut event_data = buffer.drain(..pos).collect::<Vec<u8>>(); // Estrai i dati dell'evento
                    buffer.drain(..2); // Rimuovi il delimitatore (\n\n) dal buffer

                    event_data.drain(0..5); // remove the "data: " prefix

                    let vaas = event_data.deserialize_json::<LatestVaaResponse>().unwrap();
                    println!("vaas: {:?}", vaas);

                    times += 1;
                }
            },
            Err(e) => eprintln!("Errore nel flusso: {}", e),
        }
    }

    println!("Connessione chiusa.");
}

fn find_event_delimiter(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == b"\n\n")
}

#[tokio::test]
async fn test_client() {
    let mut client = PythClient::new("https://hermes.pyth.network");
    let ids = vec![(
        "ids[]",
        "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
    )];

    let rx = client.run_streaming(ids).unwrap();

    for _ in 0..10 {
        if let Ok(vaas) = rx.try_recv() {
            println!("vaas: {:?}", vaas);
        } else {
            sleep(Duration::from_secs(1)).await;
        }
    }
}
