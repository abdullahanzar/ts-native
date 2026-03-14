export {};

let a: int = 0;
let b: int = 1;
let i: int = 0;

while (i < 10) {
  const next: int = a + b;
  a = b;
  b = next;
  i = i + 1;
}
