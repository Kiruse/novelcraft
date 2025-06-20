import React from 'react';
import cn from 'classnames';

type ButtonTypes = 'button' | 'submit' | 'reset';

export interface ButtonComponentProps {
  children: React.ReactNode;
  type: ButtonTypes | 'anchor';
  disabled: boolean;
  className: string;
  onClick?: (e: React.MouseEvent<HTMLElement>) => void;
}

export type ButtonProps = BaseButtonProps | LinkButtonProps;

interface CommonButtonProps {
  children?: React.ReactNode;
  variant?: 'filled' | 'outlined' | 'text';
  size?: 'sm' | 'md' | 'lg';
  color?: 'primary' | 'secondary' | 'success' | 'warning' | 'danger';
  disabled?: boolean;
  loading?: boolean;
  fullWidth?: boolean;
  className?: string;
  onClick?: (e: React.MouseEvent<HTMLElement>) => void;
}

export interface BaseButtonProps extends CommonButtonProps {
  type?: ButtonTypes;
  Component?: React.ComponentType<ButtonComponentProps>;
}

export interface LinkButtonProps extends CommonButtonProps {
  type: 'anchor';
  href: string;
  Component?: React.ComponentType<ButtonComponentProps>;
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = 'filled',
  type = 'button',
  size = 'md',
  color = 'primary',
  Component = Switcher,
  disabled = false,
  loading = false,
  fullWidth = false,
  className,
  onClick,
  ...props
}) => {
  const baseClasses = cn(
    'inline-flex items-center justify-center font-medium rounded-lg transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer',
    {
      'w-full': fullWidth,
    },
    // Size variants
    {
      'px-3 py-1.5 text-sm': size === 'sm',
      'px-4 py-2 text-base': size === 'md',
      'px-6 py-3 text-lg': size === 'lg',
    },
    // Color and variant combinations
    {
      // Filled variants
      'bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500': variant === 'filled' && color === 'primary',
      'bg-gray-600 text-white hover:bg-gray-700 focus:ring-gray-500': variant === 'filled' && color === 'secondary',
      'bg-green-600 text-white hover:bg-green-700 focus:ring-green-500': variant === 'filled' && color === 'success',
      'bg-yellow-600 text-white hover:bg-yellow-700 focus:ring-yellow-500': variant === 'filled' && color === 'warning',
      'bg-red-600 text-white hover:bg-red-700 focus:ring-red-500': variant === 'filled' && color === 'danger',

      // Outlined variants
      'border-2 bg-transparent border-blue-600 text-blue-600 hover:bg-blue-50 focus:ring-blue-500': variant === 'outlined' && color === 'primary',
      'border-2 bg-transparent border-gray-600 text-gray-600 hover:bg-gray-50 focus:ring-gray-500': variant === 'outlined' && color === 'secondary',
      'border-2 bg-transparent border-green-600 text-green-600 hover:bg-green-50 focus:ring-green-500': variant === 'outlined' && color === 'success',
      'border-2 bg-transparent border-yellow-600 text-yellow-600 hover:bg-yellow-50 focus:ring-yellow-500': variant === 'outlined' && color === 'warning',
      'border-2 bg-transparent border-red-600 text-red-600 hover:bg-red-50 focus:ring-red-500': variant === 'outlined' && color === 'danger',

      // Text variants
      'bg-transparent text-blue-600 hover:bg-blue-50 focus:ring-blue-500': variant === 'text' && color === 'primary',
      'bg-transparent text-gray-600 hover:bg-gray-50 focus:ring-gray-500': variant === 'text' && color === 'secondary',
      'bg-transparent text-green-600 hover:bg-green-50 focus:ring-green-500': variant === 'text' && color === 'success',
      'bg-transparent text-yellow-600 hover:bg-yellow-50 focus:ring-yellow-500': variant === 'text' && color === 'warning',
      'bg-transparent text-red-600 hover:bg-red-50 focus:ring-red-500': variant === 'text' && color === 'danger',
    },
    className,
  );

  return (
    <Component
      type={type}
      className={baseClasses}
      onClick={onClick}
      disabled={disabled || loading}
      {...props}
    >
      {loading && (
        <svg
          className="animate-spin -ml-1 mr-2 h-4 w-4"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          />
        </svg>
      )}
      {children}
    </Component>
  );
};

function Switcher({ children, type, className, ...props }: Omit<ButtonComponentProps, 'type'> & { type: ButtonTypes | 'anchor' }) {
  if (type === 'anchor') {
    return <a data-button="anchor" className={className} {...props}>{children}</a>;
  }

  return <button type={type} data-button={type} className={className} {...props}>{children}</button>;
}
