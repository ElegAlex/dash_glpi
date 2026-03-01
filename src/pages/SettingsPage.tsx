import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useInvoke } from "../hooks/useInvoke";
import type { AppConfig } from "../types/config";

function TagList({
  label,
  tags,
  onChange,
}: {
  label: string;
  tags: string[];
  onChange: (tags: string[]) => void;
}) {
  const [input, setInput] = useState("");

  const addTag = () => {
    const val = input.trim();
    if (val && !tags.includes(val)) {
      onChange([...tags, val]);
    }
    setInput("");
  };

  const removeTag = (tag: string) => {
    onChange(tags.filter((t) => t !== tag));
  };

  return (
    <div>
      <label className="block text-sm font-medium text-[#1a1f2e] mb-2">
        {label}
      </label>
      <div className="flex flex-wrap gap-2 mb-2">
        {tags.map((tag) => (
          <span
            key={tag}
            className="inline-flex items-center gap-1 px-2 py-1 rounded bg-[#eef2fb] text-[#1a1f2e] text-sm"
          >
            {tag}
            <button
              type="button"
              onClick={() => removeTag(tag)}
              className="text-[#525d73] hover:text-red-600 leading-none"
              aria-label={`Supprimer ${tag}`}
            >
              ×
            </button>
          </span>
        ))}
      </div>
      <div className="flex gap-2">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addTag();
            }
          }}
          className="flex-1 border border-[#e2e6ee] rounded px-3 py-1.5 text-sm text-[#1a1f2e] focus:outline-none focus:border-[#0C419A]"
          placeholder="Nouveau statut…"
        />
        <button
          type="button"
          onClick={addTag}
          className="px-3 py-1.5 rounded bg-[#0C419A] hover:bg-[#0a3783] text-white text-sm"
        >
          +
        </button>
      </div>
    </div>
  );
}

function NumberField({
  label,
  value,
  onChange,
  help,
}: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  help?: string;
}) {
  return (
    <div>
      <label className="block text-sm font-medium text-[#1a1f2e] mb-1">
        {label}
      </label>
      <input
        type="number"
        min={0}
        value={value}
        onChange={(e) => onChange(Math.max(0, Number(e.target.value)))}
        className="w-32 border border-[#e2e6ee] rounded px-3 py-1.5 text-sm text-[#1a1f2e] focus:outline-none focus:border-[#0C419A]"
      />
      {help && <p className="mt-1 text-xs text-[#525d73]">{help}</p>}
    </div>
  );
}

function Card({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-white border border-[#e2e6ee] rounded-lg p-6">
      <h2 className="text-base font-semibold text-[#1a1f2e] mb-4">{title}</h2>
      {children}
    </div>
  );
}

function SettingsPage() {
  const { data: config, execute: loadConfig } = useInvoke<AppConfig>();
  const [form, setForm] = useState<AppConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);

  useEffect(() => {
    loadConfig("get_config");
  }, []);

  useEffect(() => {
    if (config) setForm({ ...config });
  }, [config]);

  const set = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) => {
    setForm((prev) => (prev ? { ...prev, [key]: value } : prev));
  };

  const handleSave = async () => {
    if (!form) return;
    setSaving(true);
    setMessage(null);
    try {
      await invoke("update_config", { config: form });
      setMessage({ type: "success", text: "Configuration enregistrée" });
      loadConfig("get_config");
    } catch (err) {
      setMessage({ type: "error", text: String(err) });
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    setMessage(null);
    loadConfig("get_config");
  };

  if (!form) {
    return (
      <div className="space-y-6">
        <p className="text-sm text-[#525d73]">Chargement…</p>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <h1 className="text-xl font-semibold text-[#1a1f2e]">Paramètres</h1>

      {/* Section 1 — Seuils de charge */}
      <Card title="Seuils de charge">
        <div className="space-y-4">
          <NumberField
            label="Seuil tickets / technicien"
            value={form.seuilTicketsTechnicien}
            onChange={(v) => set("seuilTicketsTechnicien", v)}
            help="Au-delà de ce seuil, le technicien est en surcharge"
          />
          <div>
            <p className="text-sm font-medium text-[#1a1f2e] mb-2">
              Seuils de couleur de charge
            </p>
            <p className="text-xs text-[#525d73] mb-3">
              Limites pour les couleurs de charge (vert ≤ X, jaune ≤ Y, orange
              ≤ Z, rouge au-delà)
            </p>
            <div className="flex flex-wrap gap-4">
              <div>
                <label className="block text-xs font-medium text-[#525d73] mb-1">
                  Vert (≤)
                </label>
                <input
                  type="number"
                  min={0}
                  value={form.seuilCouleurVert}
                  onChange={(e) =>
                    set("seuilCouleurVert", Math.max(0, Number(e.target.value)))
                  }
                  className="w-24 border border-[#e2e6ee] rounded px-3 py-1.5 text-sm text-[#1a1f2e] focus:outline-none focus:border-[#0C419A]"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-[#525d73] mb-1">
                  Jaune (≤)
                </label>
                <input
                  type="number"
                  min={0}
                  value={form.seuilCouleurJaune}
                  onChange={(e) =>
                    set(
                      "seuilCouleurJaune",
                      Math.max(0, Number(e.target.value))
                    )
                  }
                  className="w-24 border border-[#e2e6ee] rounded px-3 py-1.5 text-sm text-[#1a1f2e] focus:outline-none focus:border-[#0C419A]"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-[#525d73] mb-1">
                  Orange (≤)
                </label>
                <input
                  type="number"
                  min={0}
                  value={form.seuilCouleurOrange}
                  onChange={(e) =>
                    set(
                      "seuilCouleurOrange",
                      Math.max(0, Number(e.target.value))
                    )
                  }
                  className="w-24 border border-[#e2e6ee] rounded px-3 py-1.5 text-sm text-[#1a1f2e] focus:outline-none focus:border-[#0C419A]"
                />
              </div>
            </div>
          </div>
        </div>
      </Card>

      {/* Section 2 — Seuils de classification */}
      <Card title="Seuils de classification">
        <div className="grid grid-cols-2 gap-4">
          <NumberField
            label="Ancienneté clôturer (jours)"
            value={form.seuilAncienneteCloture}
            onChange={(v) => set("seuilAncienneteCloture", v)}
          />
          <NumberField
            label="Inactivité clôturer (jours)"
            value={form.seuilInactiviteCloture}
            onChange={(v) => set("seuilInactiviteCloture", v)}
          />
          <NumberField
            label="Ancienneté relancer (jours)"
            value={form.seuilAncienneteRelancer}
            onChange={(v) => set("seuilAncienneteRelancer", v)}
          />
          <NumberField
            label="Inactivité relancer (jours)"
            value={form.seuilInactiviteRelancer}
            onChange={(v) => set("seuilInactiviteRelancer", v)}
          />
        </div>
      </Card>

      {/* Section 3 — Statuts */}
      <Card title="Statuts">
        <div className="space-y-6">
          <TagList
            label="Statuts vivants"
            tags={form.statutsVivants}
            onChange={(tags) => set("statutsVivants", tags)}
          />
          <TagList
            label="Statuts terminés"
            tags={form.statutsTermines}
            onChange={(tags) => set("statutsTermines", tags)}
          />
        </div>
      </Card>

      {/* Footer */}
      <div className="flex items-center gap-4">
        <button
          type="button"
          onClick={handleSave}
          disabled={saving}
          className="px-5 py-2 rounded bg-[#0C419A] hover:bg-[#0a3783] text-white text-sm font-medium disabled:opacity-50"
        >
          {saving ? "Enregistrement…" : "Enregistrer"}
        </button>
        <button
          type="button"
          onClick={handleReset}
          disabled={saving}
          className="px-5 py-2 rounded border border-[#e2e6ee] text-[#1a1f2e] text-sm font-medium hover:bg-gray-50 disabled:opacity-50"
        >
          Réinitialiser
        </button>
        {message && (
          <p
            className={`text-sm ${message.type === "success" ? "text-green-600" : "text-red-600"}`}
          >
            {message.text}
          </p>
        )}
      </div>
    </div>
  );
}

export default SettingsPage;
