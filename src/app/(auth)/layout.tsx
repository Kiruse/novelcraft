export default function AuthLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-900 py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full space-y-8">
        {children}

        <div className="text-center">
          <p className="text-sm text-gray-400">
            By signing in, you agree to our{' '}
            <a href="/terms" className="font-medium">
              Terms of Service
            </a>{' '}
            and{' '}
            <a href="/privacy" className="font-medium">
              Privacy Policy
            </a>
          </p>
        </div>
      </div>
    </div>
  );
}