import sys

def modify():
    with open("src/main.rs", "r") as f:
        content = f.read()

    # The issue where task gets cancelled might be due to `rb` being dropped when it returns or `conf.sample_rate().0 * 5` being too small
    # For a 2 second buffer at 16000 it is 32000 elements. But the microphone might be running at 44.1k or 48k.
    # If the microphone runs at 48k, 2 seconds is 96000 elements. The buffer is 32000, so it overflows and we lose data!
    # Let's fix the buffer size.
    content = content.replace("let rb = ringbuf::HeapRb::<f32>::new(16000 * 2);", "let rb = ringbuf::HeapRb::<f32>::new(conf.sample_rate().0 as usize * 5);")

    with open("src/main.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
