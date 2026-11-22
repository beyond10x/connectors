import type {FullResult, Reporter, TestCase, TestResult} from 'playwright/types/testReporter';

// Prints the ten slowest tests and the wall time of the run (~/beyond10x/AGENTS.md § Tests stay fast).
export default class SlowestReporter implements Reporter {
  private times: {title: string; ms: number; status: string}[] = [];
  private started = Date.now();

  onTestEnd(test: TestCase, result: TestResult): void {
    this.times.push({title: test.titlePath().slice(1).join(' › '), ms: result.duration, status: result.status});
  }

  onEnd(result: FullResult): void {
    const slow = [...this.times].sort((a, b) => b.ms - a.ms).slice(0, 10);
    const passed = this.times.filter((t) => t.status === 'passed').length;
    console.log('\nTen slowest tests:');
    for (const t of slow) console.log(`  ${(t.ms / 1000).toFixed(1).padStart(6)} s  ${t.status.padEnd(8)} ${t.title}`);
    console.log(`Run: ${result.status}, ${passed} of ${this.times.length} passed, wall ${((Date.now() - this.started) / 1000).toFixed(1)} s`);
  }
}
