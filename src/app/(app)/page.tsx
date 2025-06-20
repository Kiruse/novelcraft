import cn from 'classnames';
import { Button } from '../../components/Button';

export default function Home() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 via-blue-50 to-indigo-100 dark:from-slate-900 dark:via-slate-800 dark:to-slate-900">
      {/* Hero Section */}
      <section className="relative overflow-hidden">
        {/* Background decoration */}
        <div className="absolute inset-0 bg-grid-pattern opacity-5"></div>
        <div className="absolute top-0 left-1/4 w-72 h-72 bg-blue-400 rounded-full mix-blend-multiply filter blur-xl opacity-20 animate-blob"></div>
        <div className="absolute top-0 right-1/4 w-72 h-72 bg-purple-400 rounded-full mix-blend-multiply filter blur-xl opacity-20 animate-blob animation-delay-2000"></div>
        <div className="absolute -bottom-8 left-1/3 w-72 h-72 bg-pink-400 rounded-full mix-blend-multiply filter blur-xl opacity-20 animate-blob animation-delay-4000"></div>

        <div className="relative px-6 py-24 mx-auto max-w-7xl sm:py-32 lg:px-8">
          <div className="mx-auto max-w-2xl text-center">
            <div className="flex items-center justify-center mb-8">
              <div className="relative">
                <div className="absolute -inset-1 bg-gradient-to-r from-blue-600 to-purple-600 rounded-lg blur opacity-25"></div>
                <div className="relative bg-white dark:bg-slate-800 rounded-lg px-6 py-3">
                  <span className="text-sm font-medium text-gray-600 dark:text-gray-300">✨ AI-Powered Interactive Fiction</span>
                </div>
              </div>
            </div>

            <h1 className="text-4xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-6xl">
              Welcome to{' '}
              <span className="bg-gradient-to-r from-blue-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
                NovelCraft
              </span>
            </h1>

            <p className="mt-6 text-lg leading-8 text-gray-600 dark:text-gray-300 max-w-2xl mx-auto">
              Create and explore interactive stories where human creativity meets AI innovation.
              Unlike generic AI-generated content, our stories combine the best of both worlds -
              human ingenuity and AI adaptability - to create truly immersive, cohesive experiences.
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
                href="/create"
                variant="outlined"
                size="lg"
                color="primary"
                className="shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 transition-all duration-200"
              >
                Start Creating
              </Button>
            </div>

            <div className="mt-8 p-4 bg-white/50 dark:bg-slate-800/50 backdrop-blur-sm rounded-lg border border-gray-200/50 dark:border-slate-700/50">
              <p className="text-sm text-gray-600 dark:text-gray-400">
                🚀 Creation features are currently in insider preview
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="py-24 bg-white/50 dark:bg-slate-800/50 backdrop-blur-sm">
        <div className="mx-auto max-w-7xl px-6 lg:px-8">
          <div className="mx-auto max-w-2xl text-center">
            <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
              Why Choose NovelCraft?
            </h2>
            <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
              Experience the future of interactive storytelling
            </p>
          </div>

          <div className="flex flex-col mx-auto mt-16 max-w-2xl sm:mt-20 lg:mt-24 lg:max-w-none lg:gap-y-8">
            <dl className="grid max-w-xl grid-cols-1 gap-x-8 gap-y-16 lg:max-w-none lg:grid-cols-3">
              <Feature icon="🤖" name="AI-Powered Narratives">
                <p>
                  Stories that adapt and respond to your choices, creating unique experiences every time.
                </p>
              </Feature>
              <Feature icon="✍️" name="Human Creativity">
                <p>
                  Stories crafted by talented writers, enhanced by cutting-edge AI technology.
                </p>
              </Feature>
              <Feature icon="🌟" name="Immersive Experience">
                <p>
                  Dive deep into rich worlds with branching storylines and meaningful consequences.
                </p>
              </Feature>
            </dl>
            <dl className="grid max-w-xl grid-cols-1 gap-x-8 gap-y-16 lg:max-w-none lg:grid-cols-2">
              <Feature icon="🎭" name="Human-AI Collaboration">
                <p>
                  Unlike other platforms that delegate everything to AI, resulting in generic, uninspired
                  stories, our AI is your movie director, but the play is already written.
                </p>
              </Feature>
              <Feature icon="🎮" name="Embedded Game Mechanics">
                <p>
                  Not simply AI-generated text, but proper game mechanics like character sheets,
                  dice rolls, romantic relationships, and more!
                </p>
              </Feature>
            </dl>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="py-24">
        <div className="mx-auto max-w-7xl px-6 lg:px-8">
          <div className="mx-auto max-w-2xl text-center">
            <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
              Ready to Begin Your Journey?
            </h2>
            <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
              Join the next generation of dynamic storytelling and world building!
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
                Start Reading
              </Button>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

function Feature({ children, name, icon, className }: { children: React.ReactNode, name: string, icon: string, className?: string }) {
  return (
    <div key={name} className={cn("flex flex-col", className)}>
      <dt className="flex items-center gap-x-3 text-base font-semibold leading-7 text-gray-900 dark:text-white">
        <span className="text-2xl">{icon}</span>
        {name}
      </dt>
      <dd className={"mt-4 flex flex-auto flex-col text-base leading-7 text-gray-600 dark:text-gray-300"}>
        {children}
      </dd>
    </div>
  );
}
