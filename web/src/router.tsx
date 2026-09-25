import { createBrowserRouter } from 'react-router'
import Root from './routes/root.tsx'
import Home from './routes/home.tsx'
import About from './routes/about.tsx'
import Notes from './routes/notes.tsx'
import NotFound from './routes/not-found.tsx'

export const router = createBrowserRouter([
  {
    path: '/',
    Component: Root,
    children: [
      { index: true, Component: Home },
      { path: 'notes', Component: Notes },
      { path: 'about', Component: About },
      { path: '*', Component: NotFound },
    ],
  },
])
