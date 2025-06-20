import { SignedIn, SignedOut, UserButton } from "@clerk/nextjs";
import cn from "classnames";
import { Button } from './Button';
import Link from 'next/link';

export interface HeaderProps {
  variant?: 'default' | 'jumbo';
  className?: string;
}

export default function Header({ variant = 'default', className }: HeaderProps) {
  return (
    <header
      className={cn(
        'flex flex-row items-center justify-between bg-gray-800 shadow-lg p-4',
        variant,
        className
      )}
    >
      <Link href="/" className="text-white">
        <h1>NovelCraft</h1>
      </Link>
      <div className="flex flex-row items-center gap-4">
        <SignedIn>
          <Link href="/dashboard">Dashboard</Link>
          <UserButton />
        </SignedIn>
        <SignedOut>
          <Button type="anchor" href="/sign-in">
            Sign in
          </Button>
        </SignedOut>
      </div>
    </header>
  );
}
