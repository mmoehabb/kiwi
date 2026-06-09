import sys

def modify():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    # The issue: `tx.blocking_send` panics if called from inside an async runtime context.
    # We should use `tx.try_send` instead since `mpsc::Sender::try_send` is non-blocking.

    content = content.replace("tx.blocking_send(GuiEvent::RecordSample);", "tx.try_send(GuiEvent::RecordSample);")
    content = content.replace("tx.blocking_send(GuiEvent::DoneOnboarding);", "tx.try_send(GuiEvent::DoneOnboarding);")

    with open("src/gui.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
