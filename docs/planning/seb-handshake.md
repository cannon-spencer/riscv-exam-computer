# SEB handshake with Canvas

Canvas has no Safe Exam Browser support, so the exam server checks SEB's
Config Key itself and holds back the quiz's Canvas access code until the
check passes.

```
board: seb-agent --(start job + exam.seb)--> ~/exam.seb --> SEB
SEB --GET /gate/<exam>  X-SafeExamBrowser-ConfigKeyHash--> exam server
exam server: sha256(url + config key) matches? --> access code + link to Canvas
```

## Pieces

- `proto::seb_file` writes standard `.seb` files (plist, gzip, `plnd` or
  `pswd`) and computes the Config Key with SEB's algorithm. Tested against
  files saved by SEB for macOS and Windows, so official SEB on a laptop
  produces the same key.
- Each exam has a fixed random `examKeySalt`, so its key is unique and
  stable across restarts.
- With `EXAM_PUBLIC_URL` set (for example
  `https://riscv-exam-computer.download`), the `.seb` start page is
  `$EXAM_PUBLIC_URL/gate/<exam id>`. Without it, boards open Canvas
  directly and nothing is checked.
- SEB clients that expose the keys only to JavaScript (SEB 3.x on
  macOS/iOS) get a small page at the gate that posts
  `SafeExamBrowser.security.configKey` back to `/gate/<id>/verify`; the
  check is the same and the URL must be this exam's gate URL.
- `/exams/<id>/exam.seb` downloads the file for instructors (signed in).
  POSTing a password to the same URL gives a password-encrypted copy for
  official SEB on student laptops. Boards always get the unlocked file.
- On the board, seb-linux's WebKitGTK extension sends the key headers and
  enforces the URL filter (seb-linux fork, `webkit-extensions/`).

## Setting up an exam

1. In Canvas, set an access code on the quiz.
2. In the console, create the exam with the quiz URL and the same code.
3. Start it on boards. Students see the code only inside SEB on a board.

The gate protects the first hop only. The access code (or an IP filter on
the quiz) is what keeps the quiz closed to other browsers, so always set one.

## Testing on a laptop

Download the exam's .seb from the console and open it in official SEB (or a
seb-linux build). With `EXAM_PUBLIC_URL` pointing at a server the laptop can
reach, the gate should log `gate ok for exam <id> via header` (or `via js`).
