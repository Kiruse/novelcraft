import { Button } from '~/components/Button';

export default function CreatePage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 via-blue-50 to-indigo-100 dark:from-slate-900 dark:via-slate-800 dark:to-slate-900">
      {/* Hero Section */}
      <section className="relative overflow-hidden">
        {/* Background decoration */}
        <div className="absolute inset-0 bg-grid-pattern opacity-5"></div>
        <div className="absolute top-0 left-1/4 w-72 h-72 bg-blue-400 rounded-full mix-blend-multiply filter blur-xl opacity-20 animate-blob"></div>
        <div className="absolute top-0 right-1/4 w-72 h-72 bg-purple-400 rounded-full mix-blend-multiply filter blur-xl opacity-20 animate-blob animation-delay-2000"></div>

        <div className="relative px-6 py-24 mx-auto max-w-7xl sm:py-32 lg:px-8">
          <div className="mx-auto max-w-2xl text-center">
            <div className="flex items-center justify-center mb-8">
              <div className="relative">
                <div className="absolute -inset-1 bg-gradient-to-r from-blue-600 to-purple-600 rounded-lg blur opacity-25"></div>
                <div className="relative bg-white dark:bg-slate-800 rounded-lg px-6 py-3">
                  <span className="text-sm font-medium text-gray-600 dark:text-gray-300">🚧 Coming Soon</span>
                </div>
              </div>
            </div>

            <h1 className="text-4xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-6xl">
              Story Creation
            </h1>

            <p className="mt-6 text-lg leading-8 text-gray-600 dark:text-gray-300 max-w-2xl mx-auto">
              Our powerful story creation tools are currently in insider preview.
              Soon you'll be able to craft immersive interactive narratives that combine
              human creativity with AI innovation.
            </p>

            <div className="mt-10 flex items-center justify-center gap-x-6">
              <Button
                type="anchor"
                href="/stories"
                variant="filled"
                size="lg"
                color="primary"
                className="shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 transition-all duration-200"
              >
                Explore Stories
              </Button>
              <Button
                type="anchor"
                href="/"
                variant="outlined"
                size="lg"
                color="primary"
                className="shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 transition-all duration-200"
              >
                Back to Home
              </Button>
            </div>

            <div className="mt-8 p-6 bg-white/50 dark:bg-slate-800/50 backdrop-blur-sm rounded-lg border border-gray-200/50 dark:border-slate-700/50">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">
                What's Coming
              </h3>
              <ul className="text-sm text-gray-600 dark:text-gray-400 space-y-2 text-left max-w-md mx-auto">
                <li className="flex items-center gap-2">
                  <span className="text-blue-500">🖋️</span>
                  Simple, intuitive story creation editor
                </li>
                <li className="flex items-center gap-2">
                  <span className="text-blue-500">🖼️</span>
                  Upload character images to embed them in your story's generated scenes
                </li>
                <li className="flex items-center gap-2">
                  <span className="text-blue-500">🤝</span>
                  Collaborative writing tools
                </li>
                <li className="flex items-center gap-2">
                  <span className="text-blue-500">🎮</span>
                  Embedded game mechanics like character sheets, dice rolls, romantic relationships,
                  and more!
                </li>
                <li className="flex items-center gap-2">
                  <span className="text-blue-500">🎭</span>
                  Publishing and sharing platform
                </li>
                <li className="flex items-center gap-2">
                  <span className="text-blue-500">💰</span>
                  Monetize your stories
                </li>
              </ul>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
