-- Tabla de documentos de servidor con embeddings

CREATE TABLE IF NOT EXISTS server_documents (
    id SERIAL PRIMARY KEY,
    node_id VARCHAR(255) NOT NULL,
    filename VARCHAR(255) NOT NULL,
    file_type VARCHAR(50) NOT NULL,
    file_size INTEGER NOT NULL,
    content TEXT NOT NULL,
    summary TEXT,
    embedding vector(1536),
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(node_id, filename)
);

-- Añadir columnas si no existen (para migraciones incrementales)
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='server_documents' AND column_name='summary') THEN
        ALTER TABLE server_documents ADD COLUMN summary TEXT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='server_documents' AND column_name='embedding') THEN
        ALTER TABLE server_documents ADD COLUMN embedding vector(1536);
    END IF;
END $$;

