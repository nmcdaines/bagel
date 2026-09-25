import { NavLink, Outlet } from 'react-router'

export default function Root() {
  return (
    <div className="layout">
      <header>
        <strong>🥯 bagel</strong>
        <nav>
          <NavLink to="/" end>
            Home
          </NavLink>
          <NavLink to="/about">About</NavLink>
        </nav>
      </header>
      <main>
        <Outlet />
      </main>
    </div>
  )
}
