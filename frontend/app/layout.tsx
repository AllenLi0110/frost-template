import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "FROST Template",
  description: "Step-by-step 2-of-2 FROST DKG Solana wallet demo",
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
