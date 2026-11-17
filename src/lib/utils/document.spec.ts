import { describe, expect, it } from "vitest";
import {
  BRAZILIAN_UFS,
  formatDocument,
  maskCep,
  maskCnpj,
  maskCpf,
  maskPhone,
  normalizeDocument,
  normalizeForSearch,
  validateCnpj,
  validateCpf,
} from "./document";
import { customerFormSchema } from "./customer-schema";

function generateValidAlphaCnpj(base12: string): string {
  const clean = base12.toUpperCase();
  const charValue = (ch: string) => ch.charCodeAt(0) - 48;
  const w1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
  let s1 = 0;
  for (let i = 0; i < 12; i++) s1 += charValue(clean[i]) * w1[i];
  const r1 = s1 % 11;
  const dv1 = r1 < 2 ? 0 : 11 - r1;

  const w2 = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
  let s2 = 0;
  for (let i = 0; i < 12; i++) s2 += charValue(clean[i]) * w2[i];
  s2 += dv1 * w2[12];
  const r2 = s2 % 11;
  const dv2 = r2 < 2 ? 0 : 11 - r2;

  return `${clean}${dv1}${dv2}`;
}

describe("Utilitários de Documentos Brasileiros e Máscaras", () => {
  it("deve validar CPFs válidos e rejeitar sequências repetidas ou dígitos inválidos", () => {
    expect(validateCpf("52998224725")).toBe(true);
    expect(validateCpf("529.982.247-25")).toBe(true);
    expect(validateCpf("11111111111")).toBe(false);
    expect(validateCpf("000.000.000-00")).toBe(false);
    expect(validateCpf("52998224726")).toBe(false);
    expect(validateCpf("12345")).toBe(false);
  });

  it("deve validar CNPJs numéricos e alfanuméricos conforme Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024", () => {
    // CNPJ numérico tradicional
    expect(validateCnpj("11222333000181")).toBe(true);
    expect(validateCnpj("11.222.333/0001-81")).toBe(true);
    expect(validateCnpj("11222333000182")).toBe(false);
    expect(validateCnpj("00000000000000")).toBe(false);

    // CNPJ alfanumérico (Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024: ASCII - 48)
    // Exemplo oficial RFB/ENCAT: 12.ABC.345/01DE-35
    expect(validateCnpj("12.ABC.345/01DE-35")).toBe(true);
    expect(validateCnpj("12.abc.345/01de-35")).toBe(true);
    expect(validateCnpj("12.ABC.345/0001-88")).toBe(true);
    expect(validateCnpj("AB.CDE.123/0001-00")).toBe(true);
    expect(validateCnpj("12.ÁBC.345/01DE-35")).toBe(false);
    expect(validateCnpj("12.ABC.345/01DE-35$")).toBe(false);

    const alphaCnpj = generateValidAlphaCnpj("12ABC34501DE");
    expect(validateCnpj(alphaCnpj)).toBe(true);
    expect(validateCnpj(alphaCnpj.toLowerCase())).toBe(true);
    expect(validateCnpj(maskCnpj(alphaCnpj))).toBe(true);

    // Altera DV -> inválido
    const lastDigit = Number(alphaCnpj[13]);
    const tampered = `${alphaCnpj.slice(0, 13)}${(lastDigit + 1) % 10}`;
    expect(validateCnpj(tampered)).toBe(false);

    // Letras nas 2 últimas posições (DVs devem ser estritamente numéricos) -> inválido
    expect(validateCnpj("12ABC34501DEAB")).toBe(false);
  });

  it("deve aplicar máscaras dinâmicas de CPF, CNPJ alfanumérico, CEP e telefone", () => {
    expect(maskCpf("52998224725")).toBe("529.982.247-25");
    expect(maskCnpj("12abc34501de99")).toBe("12.ABC.345/01DE-99");
    // Bloqueia letras nas posições dos dígitos verificadores (pos 13-14)
    expect(maskCnpj("12ABC34501DEAB")).toBe("12.ABC.345/01DE");
    expect(maskCep("01310200")).toBe("01310-200");
    expect(maskPhone("11987654321")).toBe("(11) 98765-4321");
    expect(maskPhone("1132654321")).toBe("(11) 3265-4321");
    expect(formatDocument("52998224725", "individual")).toBe("529.982.247-25");
    expect(formatDocument("11222333000181", "legal")).toBe(
      "11.222.333/0001-81",
    );
  });

  it("deve normalizar texto removendo acentos e espaços duplicados para busca", () => {
    expect(normalizeDocument("12.abc.345/01de-00")).toBe("12ABC34501DE00");
    expect(normalizeForSearch("  São   José dos Campos  ")).toBe(
      "sao jose dos campos",
    );
    expect(BRAZILIAN_UFS).toHaveLength(27);
  });

  it("deve validar esquema Zod completo do formulário de clientes", () => {
    const alphaCnpj = generateValidAlphaCnpj("A1B2C3D40001");
    const validLegal = customerFormSchema.safeParse({
      person_type: "legal",
      name: "Locadora Alpha Eventos Ltda",
      trade_name: "Alpha Som",
      document_type: "cnpj",
      document_number: maskCnpj(alphaCnpj),
      contacts: [
        {
          name: "Ana Comercial",
          email: "ana@alpha.com.br",
          phone: "(11) 98765-4321",
          role: "Gerente",
          is_primary: true,
        },
      ],
      addresses: [
        {
          label: "Sede",
          street: "Rua Augusta",
          number: "100",
          neighborhood: "Consolação",
          city: "São Paulo",
          state: "SP",
          postal_code: "01305-000",
          is_primary: true,
        },
      ],
    });
    expect(validLegal.success).toBe(true);

    // Pré-cadastro sem documento (none), cliente estrangeiro (foreign) e cadastro sem endereço (opcional)
    const noDoc = customerFormSchema.safeParse({
      person_type: "individual",
      name: "Pré Cadastro",
      document_type: "none",
      document_number: null,
      contacts: [
        {
          name: "Contato",
          email: "contato@ex.com",
          is_primary: true,
        },
      ],
      addresses: [],
    });
    expect(noDoc.success).toBe(true);

    const foreignDoc = customerFormSchema.safeParse({
      person_type: "individual",
      name: "Foreign Client",
      document_type: "foreign",
      document_number: "USPASS998877",
      contacts: [
        {
          name: "John",
          phone: "11987654321",
          is_primary: true,
        },
      ],
      addresses: [
        {
          label: "Hotel",
          street: "Av. Atlântica",
          number: "200",
          neighborhood: "Copacabana",
          city: "Rio de Janeiro",
          state: "RJ",
          postal_code: "22021-001",
          country: "BR",
          is_primary: true,
        },
      ],
    });
    expect(foreignDoc.success).toBe(true);

    // Rejeita sem contatos (contato é obrigatório)
    const missingContact = customerFormSchema.safeParse({
      person_type: "individual",
      name: "Carlos Silva",
      document_type: "cpf",
      document_number: "529.982.247-25",
      contacts: [],
      addresses: [],
    });
    expect(missingContact.success).toBe(false);
  });
});
