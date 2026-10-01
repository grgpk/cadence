import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { api } from "../lib/api";

export const Route = createFileRoute("/register")({ component: RegisterPage });

function RegisterPage() {
  const navigate = useNavigate();
  const [form, setForm] = useState({ full_name: "", email: "", password: "" });
  const [error, setError] = useState("");
  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError("");
    try {
      await api("/api/auth/register", { method: "POST", body: JSON.stringify(form) });
      await navigate({ to: "/dashboard" });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Registration failed");
    }
  };
  return (
    <main style={{ maxWidth: 420, margin: "80px auto", padding: 24 }}>
      <div className="card">
        <h1>Create host account</h1>
        <form onSubmit={submit} style={{ display: "grid", gap: 16 }}>
          <label>
            Name
            <input
              className="input"
              value={form.full_name}
              onChange={(e) => setForm({ ...form, full_name: e.target.value })}
              required
            />
          </label>
          <label>
            Email
            <input
              className="input"
              type="email"
              value={form.email}
              onChange={(e) => setForm({ ...form, email: e.target.value })}
              required
            />
          </label>
          <label>
            Password
            <input
              className="input"
              type="password"
              minLength={8}
              value={form.password}
              onChange={(e) => setForm({ ...form, password: e.target.value })}
              required
            />
          </label>
          {error && <p style={{ color: "#b91c1c" }}>{error}</p>}
          <button className="button">Create account</button>
        </form>
        <p>
          Already host? <Link to="/login">Log in</Link>
        </p>
      </div>
    </main>
  );
}
