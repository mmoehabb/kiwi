use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;

fn main() {
    let backend = LlamaBackend::init().unwrap();
    // What is the signature of llama-cpp-2 model apply_chat_template?
}
