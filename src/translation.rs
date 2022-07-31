use tracing::info;
use anyhow::Result;

pub struct Translation {
  client: reqwest::Client
}

impl Translation {
  pub fn new() -> Self {
    Self {
      client: reqwest::Client::new(),
    }
  }

  #[tracing::instrument(skip_all, fields(
    from_lang = %from_lang, 
    to_lang = %to_lang,
    text = %text
  ))]
  pub async fn translate(&self, text: &str, from_lang: &str, to_lang: &str) -> Result<String> {
    /*
    url := "https://translate.googleapis.com/translate_a/single?client=gtx&sl=" +
		sourceLang + "&tl=" + targetLang + "&dt=t&q=" + encodedSource
 */
    let response = self.client.get("https://translate.googleapis.com/translate_a/single?client=gtx")
      .query(&[("sl",from_lang),("tl",to_lang),("dt","t"), ("q", text)])
      .send()
      .await?
      .json::<serde_json::Value>()
      .await?;

      info!("google translate response from_lang={} to_lang={} {:?}",  from_lang, to_lang,& response);

      match &response[0][0][0] {
        serde_json::Value::String(s) => {
          info!("from={} to={:?}", text, s);
          Ok(s.clone())
        },
        _ => Err(anyhow::anyhow!("google translate returned unexpected format. response_body={:?}", response))
      }
/*    
    info!("google translate api 2 response = {:?}", response);

    let body = serde_urlencoded::to_string(&serde_json::json!({
      "f.req": format!(
        "[[[\"MkEWBc\",\"[[\\\"{text}\\\",\\\"{from_lang}\\\",\\\"{to_lang}\\\",true],[null]]\",null,\"generic\"]]]", 
        text = text, 
        from_lang = from_lang, 
        to_lang = to_lang
      ),
      "at": "ADiessZXlGQBETNi8Ef8Euus0KUy:1659212606440"
    }))?;
      
    let response = self.client
    .post("https://translate.google.com/_/TranslateWebserverUi/data/batchexecute?rpcids=MkEWBc&source-path=/&f.sid=-3990940434465486761&bl=boq_translate-webserver_20220727.08_p0&hl=en&soc-app=1&soc-platform=1&soc-device=1&_reqid=1362608&rt=c")
    .header("Host", "translate.google.com")
    .header("X-Same-Domain","1")
    .header("Content-Type", "application/x-www-form-urlencoded")
    .body(body)
    .send()
    .await?;
  
    let response_text = response.text().await?;
  
    let translated_text = self.parse_message(&response_text);
  
    info!("from={} to={:?}", text, translated_text);
  
    match translated_text {
      None => {
        info!("unable to find text translation in response body");
        debug!(response_text);
        Err(anyhow::anyhow!("unable to translate text"))
      }
      Some(value) => Ok(value)
    }*/
  }

  /// Extracts translated text from Google translate response.
  #[tracing::instrument(skip_all)]
  fn parse_message(&self, input: &str) -> Option<String>{
    let mut words = Vec::new();

    let characters = input.chars().collect::<Vec<_>>();

    let mut i = 0;
    
    while i < characters.len() {
      if characters[i] == '\\' && i < characters.len() - 1 && characters[i + 1] == '"' {
        // Skip \ and "
        i += 2;

        let word_starts_at = i;

        loop {
          if i >= characters.len() || characters[i] == '\\' && i < characters.len() - 1 && characters[i + 1]== '"' {

            words.push(characters[word_starts_at..i].iter().collect::<String>());

            // Skip \ and "
            i += 2;

            break;
          }

          i += 1;
        }

       
       } else {
         i += 1;
       }
    }

    info!("parsed words. words={:?}", &words);

    if let Some(value) = words.get(2) {
      return Some(value.clone());
    }

    if let Some(value) = words.get(1) {
      return Some(value.clone())
    }

    None
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_gogle_translate_output() {
    let t = Translation::new();

    let actual = t.parse_message(r#")]}'

352
[["wrb.fr","MkEWBc","[[null,null,null,[[[0,[[[null,7]],[true]]]],7],[[\"bom dia\",null,null,7]]],[[[null,null,null,true,null,[[\"Good Morning\",null,null,null,[[\"Good Morning\",[2,5]],[\"Good day\",[2,11]]]]]]],\"en\",1,\"pt\",[\"bom dia\",\"pt\",\"en\",true]],\"pt\"]",null,null,null,"generic"],["di",50],["af.httprm",49,"-1817443582666723432",12]]
25
[["e",4,null,null,388]]
"#);

    assert_eq!(actual, Some(String::from("Good Morning")));
  }
}

