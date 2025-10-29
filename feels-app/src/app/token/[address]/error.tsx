'use client';

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <div className="container mx-auto px-4 py-8">
      <div className="bg-danger-50 border border-danger-200 rounded-lg p-6 max-w-2xl mx-auto">
        <h2 className="text-xl font-bold text-danger-800 mb-2">Something went wrong!</h2>
        <p className="text-danger-600 mb-4">{error.message}</p>
        <details className="mb-4">
          <summary className="cursor-pointer text-danger-700 hover:text-danger-800">
            View stack trace
          </summary>
          <pre className="mt-2 text-xs overflow-auto bg-danger-100 p-2 rounded">
            {error.stack}
          </pre>
        </details>
        <button
          onClick={reset}
          className="bg-danger-600 text-white px-4 py-2 rounded hover:bg-danger-700"
        >
          Try again
        </button>
      </div>
    </div>
  );
}