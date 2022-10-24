import sys

class TestWasi:
    def log(self, list: bytes) -> None:
        sys.stdout.buffer.write(list)

    def log_err(self, list: bytes) -> None:
        sys.stderr.buffer.write(list)
