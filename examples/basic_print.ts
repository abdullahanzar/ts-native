export {};

function add(a: int, b: int): int {
  return a + b;
}

function run(): void {
  console.log(add(1, 2));
  return;
}

run();