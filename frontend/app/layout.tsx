import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "FROST MPC Wallet",
  description: "2-of-2 MPC wallet dashboard for Solana Devnet transfers",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>{children}</body>
    </html>
  );
}
