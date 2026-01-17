/**
 * SDK Benchmarks
 *
 * Performance benchmarks for the CALIBER SDK.
 * Run with: bun run bench
 */

const ITERATIONS = 10000;

interface BenchResult {
  name: string;
  iterations: number;
  totalMs: number;
  avgMs: number;
  opsPerSec: number;
}

async function bench(name: string, fn: () => void | Promise<void>): Promise<BenchResult> {
  // Warmup
  for (let i = 0; i < 100; i++) {
    await fn();
  }

  const start = performance.now();
  for (let i = 0; i < ITERATIONS; i++) {
    await fn();
  }
  const end = performance.now();

  const totalMs = end - start;
  const avgMs = totalMs / ITERATIONS;
  const opsPerSec = Math.round(1000 / avgMs);

  return { name, iterations: ITERATIONS, totalMs, avgMs, opsPerSec };
}

function formatResult(result: BenchResult): string {
  return `${result.name}: ${result.opsPerSec.toLocaleString()} ops/sec (${result.avgMs.toFixed(4)}ms avg)`;
}

async function runBenchmarks() {
  const results: BenchResult[] = [];

  console.log('Running CALIBER SDK benchmarks...\n');

  // Benchmark: Object creation
  results.push(
    await bench('Object creation', () => {
      const obj = {
        id: 'test-id',
        name: 'test-name',
        data: { nested: { value: 42 } },
      };
      return obj;
    })
  );

  // Benchmark: JSON serialization
  const testObject = {
    id: 'trajectory-123',
    name: 'Test Trajectory',
    scopes: [
      { id: 'scope-1', artifacts: [{ id: 'art-1', type: 'code', tokens: 1000 }] },
      { id: 'scope-2', artifacts: [{ id: 'art-2', type: 'text', tokens: 500 }] },
    ],
  };

  results.push(
    await bench('JSON.stringify', () => {
      JSON.stringify(testObject);
    })
  );

  results.push(
    await bench('JSON.parse', () => {
      JSON.parse('{"id":"test","name":"value","data":[1,2,3]}');
    })
  );

  // Benchmark: Array operations
  const largeArray = Array.from({ length: 1000 }, (_, i) => ({
    id: `item-${i}`,
    value: i,
  }));

  results.push(
    await bench('Array.filter', () => {
      largeArray.filter((item) => item.value > 500);
    })
  );

  results.push(
    await bench('Array.map', () => {
      largeArray.map((item) => ({ ...item, doubled: item.value * 2 }));
    })
  );

  results.push(
    await bench('Array.reduce', () => {
      largeArray.reduce((sum, item) => sum + item.value, 0);
    })
  );

  // Benchmark: String operations
  results.push(
    await bench('String interpolation', () => {
      const id = 'test-id';
      const name = 'test-name';
      return `Trajectory ${id}: ${name}`;
    })
  );

  results.push(
    await bench('String.split', () => {
      'tenant-abc:trajectory-123:scope-456'.split(':');
    })
  );

  // Benchmark: URL construction
  results.push(
    await bench('URL construction', () => {
      new URL('/api/v1/trajectories', 'https://api.caliber.run');
    })
  );

  // Benchmark: Map operations
  const testMap = new Map<string, number>();
  for (let i = 0; i < 1000; i++) {
    testMap.set(`key-${i}`, i);
  }

  results.push(
    await bench('Map.get', () => {
      testMap.get('key-500');
    })
  );

  results.push(
    await bench('Map.set', () => {
      testMap.set('new-key', 999);
    })
  );

  // Print results
  console.log('Results:\n');
  for (const result of results) {
    console.log(formatResult(result));
  }

  // JSON output for CI
  if (process.argv.includes('--json')) {
    console.log('\nJSON Output:');
    console.log(JSON.stringify(results, null, 2));
  }

  return results;
}

runBenchmarks().catch(console.error);
