import { createBrowserRouter } from 'react-router'
import Root from './routes/root.tsx'
import Home from './routes/home.tsx'
import { fetchHealth } from './api.ts'
import About from './routes/about.tsx'
import NotFound from './routes/not-found.tsx'

export const router = createBrowserRouter([
  {
    path: '/',
    Component: Root,
    children: [
      { index: true, Component: Home, loader: fetchHealth },
      { path: 'about', Component: About },
      { path: '*', Component: NotFound },
    ],
  },
])
