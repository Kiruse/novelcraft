import { SignIn } from '@clerk/nextjs';

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
        <SignIn
          forceRedirectUrl="/post-sign-in"
          signUpUrl="/sign-up"
          appearance={{
            elements: {
              formButtonPrimary:
                'bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-lg transition-colors duration-200',
              card: 'shadow-none bg-transparent',
              headerTitle: 'text-xl font-semibold text-white',
              headerSubtitle: 'text-gray-300',
              socialButtonsBlockButton:
                'bg-gray-700 border border-gray-600 text-gray-200 hover:bg-gray-600 font-medium py-2 px-4 rounded-lg transition-colors duration-200',
              formFieldInput:
                'block w-full px-3 py-2 border border-gray-600 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-gray-700 text-white placeholder-gray-400',
              formFieldLabel: 'block text-sm font-medium text-gray-200 mb-1',
              footerActionLink: 'text-blue-400 hover:text-blue-300 font-medium',
              dividerLine: 'bg-gray-600',
              dividerText: 'text-gray-400 bg-gray-800 px-2',
              formFieldError: 'text-red-400',
              formFieldErrorText: 'text-red-400 text-sm',
              footer: 'text-gray-400',
              footerAction: 'text-gray-400',
              identityPreviewText: 'text-gray-200',
              identityPreviewEditButton: 'text-blue-400 hover:text-blue-300',
              formResendCodeLink: 'text-blue-400 hover:text-blue-300',
              otpCodeFieldInput: 'bg-gray-700 border-gray-600 text-white',
              userButtonPopoverCard: 'bg-gray-800 border-gray-700',
              userButtonPopoverActionButton: 'text-gray-200 hover:bg-gray-700',
              userButtonPopoverActionButtonText: 'text-gray-200',
              userButtonPopoverFooter: 'text-gray-400',
              userButtonPopoverFooterAction: 'text-gray-400',
            },
          }}
        />
      </div>
    </>
  );
}
