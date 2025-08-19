export default function SignInPage() {
  return (
    <>
      <div className="text-center">
        <h1 className="text-3xl font-bold text-white mb-2">
          Welcome to NovelCraft
        </h1>
        <p className="text-gray-300">
          Sign in to your account to continue
        </p>
      </div>

      <div className="bg-gray-800 py-8 px-6 shadow-xl rounded-lg border border-gray-700">
        <div className="text-center">
          <h2 className="text-xl font-semibold text-white mb-4">
            Sign In
          </h2>
          <p className="text-gray-400 mb-6">
            Authentication system coming soon...
          </p>
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-200 mb-1">
                Email
              </label>
              <input
                type="email"
                className="block w-full px-3 py-2 border border-gray-600 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-gray-700 text-white placeholder-gray-400"
                placeholder="Enter your email"
                disabled
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-200 mb-1">
                Password
              </label>
              <input
                type="password"
                className="block w-full px-3 py-2 border border-gray-600 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-gray-700 text-white placeholder-gray-400"
                placeholder="Enter your password"
                disabled
              />
            </div>
            <button
              className="w-full bg-gray-600 text-gray-300 font-medium py-2 px-4 rounded-lg cursor-not-allowed"
              disabled
            >
              Sign In
            </button>
          </div>
          <p className="text-gray-500 mt-4">
            Don't have an account?{' '}
            <a href="/sign-up" className="text-blue-400 hover:text-blue-300 font-medium">
              Sign up
            </a>
          </p>
        </div>
      </div>
    </>
  );
}
