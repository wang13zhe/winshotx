/* eslint-disable no-console */
// 校验脚本：检查三个语言字典与 base-es-en.json 的一致性。
// 用完即删。
const fs = require("node:fs");
const path = require("node:path");

const RAIZ = "C:/Users/Admin/code/winshotx";
const ts = require(path.join(RAIZ, "node_modules/typescript"));

const ARCHIVOS = {
  "pt-br": path.join(RAIZ, "src/lib/textos-pt-br.ts"),
  "zh-tw": path.join(RAIZ, "src/lib/textos-zh-tw.ts"),
  pl: path.join(RAIZ, "src/lib/textos-pl.ts"),
};

const base = JSON.parse(fs.readFileSync(path.join(RAIZ, "docs/base-es-en.json"), "utf8"));
const baseKeys = Object.keys(base);

// 不构成其他词子串的保护词，按子串查。
const PROTEGIDAS = [
  "Impr Pant",
  "GitHub",
  "Microsoft Store",
  "PNG",
  "GIF",
  "MP4",
  "FFmpeg",
  "MIT",
  "H.264",
];
// Win/Esc/Ctrl/Shift 会构成其他词的子串（Windows、Escribe），必须按词边界查。
const PROTEGIDAS_EXACTAS = ["Esc", "Win", "Ctrl", "Shift"];

// 台湾繁体：不允许出现的大陆用语/简体字。
// 注意：「段」本身是台灣「區段」的常用字，不能单字查；只查成詞的。
const MANDARIN_SIMPLIFICADO = [
  "截图", "录屏", "设置", "剪贴板", "视频", "软件", "鼠标", "保存", "文件夹",
  "默认", "质量", "撤销", "资源管理器", "点击", "粘贴", "录影", "屏幕", "键盘",
  "激活", "重启", "网络", "数据", "上传", "下载", "对话框", "选项", "确认",
  "语言包", "高清", "解码", "缓存", "用户", "登录", "注销", "隐私", "时间线",
  "帧率", "分辨率", "编码", "开启", "关闭", "启动", "退出", "复制",
  "简体", "繁体", "中文", "信息", "文档", "编辑", "视图", "帮助",
  "菜单栏", "右键", "左键", "双击", "拖拽", "截屏", "全屏", "窗口化",
  // 「工具列」是台灣正確講法，只查大陸的「工具栏」
  "工具栏", "任务栏", "选项卡", "默认值", "文件名", "快捷方式",
];
// 简体字特征字（在繁体中文里几乎不单独出现）
const HAN_SIMPLIFICADO_PURO = [
  "个", "张", "们", "这", "谁", "没", "还", "发", "后", "里", "为", "与",
  "无", "学", "写", "实", "宝", "对", "双", "观", "览", "见", "视", "频",
  "译", "语", "说", "话", "请", "谢", "赖", "赛", "贝", "贵", "赞", "车",
  "轨", "转", "输", "边", "过", "连", "进", "运", "选", "逐", "递", "闭",
  "门", "问", "开", "关", "闪", "队", "阴", "阳", "陆", "险", "随", "难",
  "静", "预", "顶", "页", "项", "顺", "频", "译", "铁", "银", "图", "国",
];

// 巴西葡语：西语化借词/错误词。
const ESPANHOLISMO = [
  "Grabar", "coger", "Coger", "ratón", "Ratón", "ordenador", "Ordenador",
  "pantalla", "Pantalla", "ajustes", "Ajustes", "portapapeles", "Portapapeles",
  "archivo", "Archivo", "fichero", "Fichero", "imagen", "Imagen",
];

function extraer(texto, nombreVar) {
  const src = ts.createSourceFile(
    nombreVar + ".ts",
    texto,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  let salida = null;
  src.statements.forEach((st) => {
    if (ts.isVariableStatement(st)) {
      st.declarationList.declarations.forEach((d) => {
        if (ts.isIdentifier(d.name) && d.name.text === nombreVar && d.initializer) {
          salida = d.initializer;
        }
      });
    }
  });
  if (!salida) throw new Error("no se encontró " + nombreVar);
  const obj = {};
  salida.properties.forEach((p) => {
    if (ts.isPropertyAssignment(p)) {
      const k = p.name.text ?? JSON.parse(p.name.getText(src));
      obj[k] = p.initializer.text;
    } else if (ts.isShorthandPropertyAssignment(p)) {
      obj[p.name.text] = p.name.text;
    }
  });
  return obj;
}

function placeholders(s) {
  return (s.match(/\{[a-zA-ZáéíóúñÁÉÍÓÚÑ_]+\}/g) || []).sort();
}

let fallosTotales = 0;

for (const [lang, file] of Object.entries(ARCHIVOS)) {
  const texto = fs.readFileSync(file, "utf8");
  const varName = { "pt-br": "PT_BR", "zh-tw": "ZH_TW", pl: "PL" }[lang];
  const dict = extraer(texto, varName);
  const keys = Object.keys(dict);
  const problemas = [];

  if (keys.length !== baseKeys.length) {
    problemas.push(`条目数 ${keys.length} != base ${baseKeys.length}`);
  }
  const faltan = baseKeys.filter((k) => !(k in dict));
  const sobran = keys.filter((k) => !(k in base));
  if (faltan.length) problemas.push(`缺键 ${faltan.length}: ${JSON.stringify(faltan.slice(0, 8))}`);
  if (sobran.length) problemas.push(`多键 ${sobran.length}: ${JSON.stringify(sobran.slice(0, 8))}`);

  for (const k of baseKeys) {
    if (!(k in dict)) continue;
    const a = placeholders(k);
    const b = placeholders(dict[k]);
    if (JSON.stringify(a) !== JSON.stringify(b)) {
      problemas.push(`占位符不一致 [${k.slice(0, 45)}] 原文${JSON.stringify(a)} 译文${JSON.stringify(b)}`);
    }
  }

  for (const k of baseKeys) {
    if (!(k in dict)) continue;
    const v = dict[k];
    const oC = (k.match(/winshotx/g) || []).length;
    const oV = (v.match(/winshotx/g) || []).length;
    if (oC !== oV) problemas.push(`winshotx 数量不符 [${k.slice(0, 45)}] ${oC}->${oV}`);
    if (/Winshotx|WINSHOTX/.test(v)) problemas.push(`winshotx 大小写错误 [${k.slice(0, 45)}]`);
  }

  for (const k of baseKeys) {
    if (!(k in dict)) continue;
    const v = dict[k];
    // Win 是 Windows 的子串，Esc 是 Escribe 的子串：全部按词级查，避免误报。
    for (const p of PROTEGIDAS_EXACTAS) {
      const re = new RegExp(`(?<![A-Za-zÁÉÍÓÚÑáéíóúñ])${p}(?![A-Za-zÁÉÍÓÚÑáéíóúñ])`);
      if (re.test(k) && !re.test(v)) {
        problemas.push(`保护词丢失 [${k.slice(0, 45)}] 缺 "${p}"`);
      }
    }
    // 其余保护词不构成其他词的子串，按子串查即可。
    for (const p of PROTEGIDAS) {
      if (k.includes(p) && !v.includes(p)) {
        problemas.push(`保护词丢失 [${k.slice(0, 45)}] 缺 "${p}"`);
      }
    }
  }

  if (lang === "zh-tw") {
    for (const k of baseKeys) {
      if (!(k in dict)) continue;
      const v = dict[k];
      for (const bad of MANDARIN_SIMPLIFICADO) {
        if (v.includes(bad)) problemas.push(`大陆用语 [${k.slice(0, 40)}] "${bad}"`);
      }
      for (const ch of HAN_SIMPLIFICADO_PURO) {
        if (v.includes(ch)) problemas.push(`简体字疑? [${k.slice(0, 40)}] "${ch}"`);
      }
    }
  }
  if (lang === "pt-br") {
    for (const k of baseKeys) {
      if (!(k in dict)) continue;
      const v = dict[k];
      for (const bad of ESPANHOLISMO) {
        if (v.includes(bad)) problemas.push(`西语化词汇 [${k.slice(0, 40)}] "${bad}"`);
      }
      // tu vs você
      const usosTu = (v.match(/(?<![A-Za-zÀ-ÿ])tu(?![A-Za-zÀ-ÿ])/g) || []).length;
      if (usosTu > 0) problemas.push(`用了 tu 而非 você [${k.slice(0, 40)}]`);
    }
  }

  console.log("=".repeat(70));
  console.log(`${lang} (${varName}): ${keys.length} 条`);
  if (problemas.length === 0) {
    console.log("  ✅ 无问题");
  } else {
    console.log(`  ❌ ${problemas.length} 个问题:`);
    problemas.forEach((p) => console.log("    - " + p));
  }
  fallosTotales += problemas.length;
}
console.log("=".repeat(70));
console.log(fallosTotales === 0 ? "全部通过" : `合计 ${fallosTotales} 个问题`);
