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
      <div className="bg-red-50 border border-red-200 rounded-lg p-6 max-w-2xl mx-auto">
        <h2 className="text-xl font-bold text-red-800 mb-2">Something went wrong!</h2>
        <p className="text-red-600 mb-4">{error.message}</p>
        <details className="mb-4">
          <summary className="cursor-pointer text-red-700 hover:text-red-800">
            View stack trace
          </summary>
          <pre className="mt-2 text-xs overflow-auto bg-red-100 p-2 rounded">
            {error.stack}
          </pre>
        </details>
        <button
          onClick={reset}
          className="bg-red-600 text-white px-4 py-2 rounded hover:bg-red-700"
        >
          Try again
        </button>
      </div>
    </div>
  );
}