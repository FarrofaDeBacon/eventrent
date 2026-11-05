# ADR-0006: Dois Modos de Operação (Local e Nuvem) e Configuração Pré-Banco

## Contexto

O EventRent atende desde empresas de locação que operam em um único computador no galpão (sem dependência de internet) até empresas com múltiplos computadores e equipes simultâneas que necessitam de um servidor centralizado. Além disso, empresas que iniciam no modo local devem poder migrar seus dados integralmente para o modo nuvem no futuro sem perda de histórico ou conflito de chaves.

## Decisão

1. **Dois Modos de Operação na Instalação**:
   - **Modo Local ("Neste computador")**: Banco de dados SQLite local na máquina do usuário, operação 100% offline, implementado a partir da Fase 1A.
   - **Modo Nuvem ("Em mais de um computador")**: Servidor central (`axum`) com PostgreSQL compartilhado entre múltiplos clientes desktop/web, a ser implementado na Fase 2.5 (exibido como opção "Em breve" no assistente de primeira execução da Fase 1A).
2. **Persistência da Escolha de Modo Fora do Banco (`config.json`)**:
   - O modo escolhido é gravado em `<app_data_dir>/config.json`, lido na inicialização do aplicativo **antes** da abertura do banco de dados, permitindo que o runtime saiba se deve abrir o SQLite local, exibir o assistente de primeira execução ou conectar-se ao servidor remoto.
3. **Chave de Recuperação Offline no Modo Local**:
   - Como o modo local opera sem servidor de e-mail garantido, a criação da conta do Proprietário gera uma **Chave de Recuperação** única (`ER-XXXX-XXXX-XXXX-XXXX`), exibida uma única vez e armazenada apenas como hash Argon2id no banco.
4. **Preparação para Migração Local -> Nuvem (Fase 2.6)**:
   - Todas as entidades utilizam `id` em UUID v7 e `tenant_id` obrigatório mesmo no modo local, garantindo que a migração futura para PostgreSQL ocorra por cópia direta de registros sem colisão de chaves primárias.

## Consequências

- **Positivas**: Flexibilidade comercial imediata (atende clientes pequenos offline e operações maiores em rede) e caminho de upgrade sem lock-in.
- **Negativas/Atenção**: Requer que `config.json` seja validado antes de qualquer conexão de banco na inicialização.
