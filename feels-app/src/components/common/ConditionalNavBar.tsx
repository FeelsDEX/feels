'use client';

import { NavBar } from './NavBar';

export function ConditionalNavBar() {
  // Always show the NavBar - it handles hiding the search component internally
  return <NavBar />;
}