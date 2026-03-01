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
      <label className="block text-sm font-medium text-slate-800 mb-2">
        {label}
      </label>
      <div className="flex flex-wrap gap-2 mb-2">
        {tags.map((tag) => (
          <span
            key={tag}
            className="inline-flex items-center gap-1 px-2 py-1 rounded-lg bg-primary-50 text-slate-800 text-sm"
          >
            {tag}
            <button
              type="button"
              onClick={() => removeTag(tag)}
              className="text-slate-500 hover:text-danger-500 leading-none"
              aria-label={`Supprimer ${tag}`}
            >
              x
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
          className="flex-1 rounded-lg bg-white px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
          placeholder="Nouveau statut..."
        />
        <button
          type="button"
          onClick={addTag}
          className="px-3 py-1.5 rounded-lg bg-primary-500 hover:bg-primary-600 text-white text-sm shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
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
      <label className="block text-sm font-medium text-slate-800 mb-1">
        {label}
      </label>
      <input
        type="number"
        min={0}
        value={value}
        onChange={(e) => onChange(Math.max(0, Number(e.target.value)))}
        className="w-32 rounded-lg bg-white px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
      />
      {help && <p className="mt-1 text-xs text-slate-400">{help}</p>}
    </div>
  );
}

function SettingsCard({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-6">
      <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-4">{title}</h2>
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
      setMessage({ type: "success", text: "Configuration enregistree" });
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
      <div>
        <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
          <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
            Parametres
          </h1>
          <p className="text-sm text-slate-400 mt-1">
            Configuration des seuils et statuts
          </p>
        </header>
        <div className="px-8 pb-8 pt-6">
          <p className="text-sm text-slate-400">Chargement...</p>
        </div>
      </div>
    );
  }

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Parametres
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Configuration des seuils et statuts
        </p>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6 max-w-3xl">
        {/* Section 1 — Seuils de charge */}
        <div className="animate-fade-slide-up">
        <SettingsCard title="Seuils de charge">
          <div className="space-y-4">
            <NumberField
              label="Seuil tickets / technicien"
              value={form.seuilTicketsTechnicien}
              onChange={(v) => set("seuilTicketsTechnicien", v)}
              help="Au-dela de ce seuil, le technicien est en surcharge"
            />
            <div>
              <p className="text-sm font-medium text-slate-800 mb-2">
                Seuils de couleur de charge
              </p>
              <p className="text-xs text-slate-400 mb-3">
                Limites pour les couleurs de charge (vert &lt;= X, jaune &lt;= Y, orange
                &lt;= Z, rouge au-dela)
              </p>
              <div className="flex flex-wrap gap-4">
                <div>
                  <label className="block text-xs font-medium text-slate-500 mb-1">
                    Vert (&lt;=)
                  </label>
                  <input
                    type="number"
                    min={0}
                    value={form.seuilCouleurVert}
                    onChange={(e) =>
                      set("seuilCouleurVert", Math.max(0, Number(e.target.value)))
                    }
                    className="w-24 rounded-lg bg-white px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium text-slate-500 mb-1">
                    Jaune (&lt;=)
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
                    className="w-24 rounded-lg bg-white px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium text-slate-500 mb-1">
                    Orange (&lt;=)
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
                    className="w-24 rounded-lg bg-white px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
                  />
                </div>
              </div>
            </div>
          </div>
        </SettingsCard>
        </div>

        {/* Section 2 — Seuils de classification */}
        <div className="animate-fade-slide-up animation-delay-150">
        <SettingsCard title="Seuils de classification">
          <div className="grid grid-cols-2 gap-4">
            <NumberField
              label="Anciennete cloturer (jours)"
              value={form.seuilAncienneteCloture}
              onChange={(v) => set("seuilAncienneteCloture", v)}
            />
            <NumberField
              label="Inactivite cloturer (jours)"
              value={form.seuilInactiviteCloture}
              onChange={(v) => set("seuilInactiviteCloture", v)}
            />
            <NumberField
              label="Anciennete relancer (jours)"
              value={form.seuilAncienneteRelancer}
              onChange={(v) => set("seuilAncienneteRelancer", v)}
            />
            <NumberField
              label="Inactivite relancer (jours)"
              value={form.seuilInactiviteRelancer}
              onChange={(v) => set("seuilInactiviteRelancer", v)}
            />
          </div>
        </SettingsCard>
        </div>

        {/* Section 3 — Text Mining */}
        <div className="animate-fade-slide-up animation-delay-300">
        <SettingsCard title="Text Mining">
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-slate-800 mb-1">
                Seuil de similarite doublons (Jaro-Winkler)
              </label>
              <input
                type="number"
                min={0.5}
                max={1}
                step={0.01}
                value={form.seuilSimilariteDoublons}
                onChange={(e) => {
                  const v = Math.min(1, Math.max(0.5, Number(e.target.value)));
                  set("seuilSimilariteDoublons", v);
                }}
                className="w-32 rounded-lg bg-white px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
              />
              <p className="mt-1 text-xs text-slate-400">
                Plus le seuil est eleve, moins il y aura de doublons detectes (defaut : 0.92)
              </p>
            </div>
          </div>
        </SettingsCard>
        </div>

        {/* Section 4 — Statuts */}
        <div className="animate-fade-slide-up animation-delay-450">
        <SettingsCard title="Statuts">
          <div className="space-y-6">
            <TagList
              label="Statuts vivants"
              tags={form.statutsVivants}
              onChange={(tags) => set("statutsVivants", tags)}
            />
            <TagList
              label="Statuts termines"
              tags={form.statutsTermines}
              onChange={(tags) => set("statutsTermines", tags)}
            />
          </div>
        </SettingsCard>
        </div>

        {/* Footer */}
        <div className="animate-fade-slide-up animation-delay-450 flex items-center gap-4">
          <button
            type="button"
            onClick={handleSave}
            disabled={saving}
            className="px-5 py-2 rounded-xl bg-primary-500 hover:bg-primary-600 text-white text-sm font-medium disabled:opacity-50 shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
          >
            {saving ? "Enregistrement..." : "Enregistrer"}
          </button>
          <button
            type="button"
            onClick={handleReset}
            disabled={saving}
            className="px-5 py-2 rounded-xl bg-white text-slate-800 text-sm font-medium hover:bg-slate-50 disabled:opacity-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
          >
            Reinitialiser
          </button>
          {message && (
            <p
              className={`text-sm ${message.type === "success" ? "text-success-500" : "text-danger-500"}`}
            >
              {message.text}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

export default SettingsPage;
