use std::io;
use std::io::Write;

use serde::{Deserialize, Serialize};
use reqwest;
use clap::Parser;

static PREAMBLE: &str = "This is a conversation between a {language} tutor and a {language} learner. The tutor will correct any mistakes in the students grammar.\n\n\n";
static INTERACTION_PROMPT: &str = "S: {question}\nT:";
static INTERACTION: &str = "S: {question}\nT: {response}\n\n";
const MAX_TOKENS: usize = 256;

#[derive(Parser)]
struct Cli {
    token: String,
    #[clap(default_value_t = String::from("Mandarin Chinese"))]
    language: String,
}

#[derive(Debug, Serialize)]
struct GptRequest {
    model: &'static str,
    stop: &'static str,

    prompt: String,
    temperature: f64,
    max_tokens: usize
}
impl GptRequest {
    pub fn basic(prompt: String, temperature: f64, stop: &'static str) -> Self {
        GptRequest {
            model: "text-davinci-002",
            max_tokens: MAX_TOKENS,
            stop, prompt, temperature
        }
    }
}
#[derive(Debug, Deserialize)]
struct GptChoice {
    text: String,
}
#[derive(Debug, Deserialize)]
struct GptResponse {
    choices: Vec<GptChoice>
}

#[derive(Debug)]
struct Interaction {
    pub question: String,
    pub response: String
}
impl Interaction {
    pub fn to_string(&self) -> String {
        String::from(INTERACTION)
            .replace("{question}", &self.question)
            .replace("{response}", &self.response)
    }
}

#[derive(Debug)]
struct Conversation {
    pub language: String,
    pub interactions: Vec<Interaction>
}
impl Conversation {
    pub fn new(language: String) -> Self {
        Conversation {
            language,
            interactions: Vec::new()
        }
    }

    pub fn ask(&self, question: &str, context: usize) -> GptRequest {
        let mut request = String::from(PREAMBLE).replace("{language}", &self.language);
        for interaction in self.interactions.iter().rev().take(context).rev() {
            request.push_str(&interaction.to_string());
        }
        request.push_str(&String::from(INTERACTION_PROMPT)
                         .replace("{question}", question)
                         .replace("{response}", ""));
        GptRequest::basic(request, 0.8, "S:")
    }
    pub fn process_response(&mut self, question: impl Into<String>, response: impl Into<String>) {
        self.interactions.push(Interaction {
            question: question.into(),
            response: response.into()
        });
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut conversation = Conversation::new(cli.language);

    print!("{}", String::from(PREAMBLE).replace("{language}", &conversation.language));

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let client = reqwest::Client::new();
    loop {
        print!("> ");
        stdout.flush();

        let mut input = String::new();
        stdin.read_line(&mut input);
        let input = input.trim();

        let query = serde_json::to_string(&conversation.ask(input, 10)).unwrap();

        //println!("{:?}", query);

        let response_text = client.post("https://api.openai.com/v1/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", &cli.token))
            .body(query)
            .send()
            .await.unwrap().text().await.unwrap();

        //println!("{:?}", response_text);

        let mut response: GptResponse = serde_json::from_str(&response_text).unwrap();

        //println!("{:?}", response);

        let answer = response.choices.pop().unwrap().text;
        let answer = answer.trim();

        conversation.process_response(input, answer);
        println!("The teacher says: {}", answer);
        println!("");
    }
}
