export function findsInArray(xs: string[], t: string) {
  return xs.filter(x => x === t)[0];
}

interface JerkCode<T> {
  filter(predicate: (item: T) => boolean): JerkCode<T>;
}

export function looksTheSame(jerk: JerkCode<string>) {
  return jerk.filter(item => item === "aha")[0];
}

export function helper(n: number): number {
  return n * 2;
}

export function callsHelper(n: number): number {
  return helper(n);
}
