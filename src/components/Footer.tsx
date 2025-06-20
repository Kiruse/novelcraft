import React from 'react';
import cn from 'classnames';
import Link from 'next/link';

export interface FooterProps {
  variant?: 'default' | 'jumbo';
  className?: string;
};

export const Footer: React.FC<FooterProps> = ({
  variant = 'default',
  className,
}) => {
  const currentYear = new Date().getFullYear();

  return (
    <footer className={cn('w-full py-6 px-4 mt-auto bg-gray-800', variant, className)}>
      <div className="container mx-auto flex flex-col sm:flex-row justify-between items-center text-sm text-gray-400">
        <div>
          © {currentYear} kirudev. All rights reserved.
        </div>
        <div className="mt-4 sm:mt-0 space-x-4">
          <Link
            href="/terms"
            className="hover:text-gray-900 transition-colors"
          >
            Terms of Service
          </Link>
          <span>·</span>
          <Link
            href="/privacy"
            className="hover:text-gray-900 transition-colors"
          >
            Privacy Policy
          </Link>
        </div>
      </div>
    </footer>
  );
};