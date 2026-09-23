import { useState } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import Nav from './Nav';
import PageHero from './PageHero';
import { useAuthStore } from '../stores/authStore';
import { heroForPath } from '../navConfig';
import { applyTheme, readStoredTheme, toggleTheme, type Theme } from '../theme';

export default function MainLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const logout = useAuthStore((s) => s.logout);
  const [theme, setTheme] = useState<Theme>(() => {
    const t = readStoredTheme();
    applyTheme(t);
    return t;
  });

  const isOverview = location.pathname === '/';
  const hero = isOverview ? null : heroForPath(location.pathname);

  return (
    <>
      <Nav
        theme={theme}
        onToggleTheme={() => setTheme((t) => toggleTheme(t))}
        onLogout={() => {
          logout();
          navigate('/');
        }}
      />
      <main>
        <div key={location.pathname}>
          {isOverview ? (
            <header className="hero">
              <div>
                <p className="eyebrow">CILIUM-NATIVE OBSERVABILITY</p>
                <h1>Trace every flow.</h1>
                <p>
                  See where network traffic goes and why it is allowed or dropped — powered by Cilium eBPF. Paqtra
                  observes; Cilium decides.
                </p>
              </div>
            </header>
          ) : (
            hero && <PageHero eyebrow={hero.eyebrow} title={hero.title} lede={hero.lede} tint={hero.tint} />
          )}
          <div className={isOverview ? undefined : 'view-board'}>
            <Outlet />
          </div>
        </div>
      </main>
    </>
  );
}
