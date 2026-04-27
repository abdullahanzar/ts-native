export {};

function add(a: int, b: int): int {
  const total: int = a + b;
  return total;
}

function run(): int {
  const first: int = add(1, 2);
  add(first, 3);
  return add(first, 4);
}

function finish(): void {
  run();
  return;
}