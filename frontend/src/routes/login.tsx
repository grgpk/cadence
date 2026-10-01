import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { api } from "../lib/api";

export const Route = createFileRoute("/login")({ component: LoginPage });

function LoginPage() {
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError("");
    try {
      await api("/api/auth/login", {
        method: "POST",
        body: JSON.stringify({ email, password }),
      });
      await navigate({ to: "/dashboard" });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Login failed");
    }
  };
  return (
    <main style={{ maxWidth: 420, margin: "80px auto", padding: 24 }}>
      <div className="card">
        <h1>Log in</h1>
        <form onSubmit={submit} style={{ display: "grid", gap: 16 }}>
          <label>
            Email
            <input
              className="input"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              type="email"
              required
            />
          </label>
          <label>
            Password
            <input
              className="input"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              type="password"
              required
            />
          </label>
          {error && <p style={{ color: "#b91c1c" }}>{error}</p>}
          <button className="button">Log in</button>
        </form>
        <p>
          No account? <Link to="/register">Create host account</Link>
        </p>
      </div>
    </main>
  );
}
