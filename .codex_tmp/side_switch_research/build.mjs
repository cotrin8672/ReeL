import fs from "node:fs/promises";
import path from "node:path";
import { SpreadsheetFile, Workbook } from "@oai/artifact-tool";

const outDir = path.resolve("../../outputs/side_push_switch_research_20260718");
await fs.mkdir(outDir, { recursive: true });

const urls = {
  alpsSksj: "https://tech.alpsalpine.com/e/products/category/tact-switch/sub/02/series/sksj/",
  alpsSksc: "https://tech.alpsalpine.com/e/products/category/tact-switch/sub/02/series/sksc/",
  alpsSktd: "https://tech.alpsalpine.com/e/products/category/tact-switch/sub/02/series/sktd/",
  alpsSksl: "https://tech.alpsalpine.com/e/products/category/tact-switch/sub/02/series/sksl/",
  alpsSkrt: "https://tech.alpsalpine.com/e/products/category/tact-switch/sub/02/series/skrt/",
  alpsSksn: "https://tech.alpsalpine.com/e/products/category/tact-switch/sub/02/series/sksn/",
  panaP7: "https://industry.panasonic.com/global/en/products/control/switch/light-touch/3529m_smd_side",
  panaPu: "https://industry.panasonic.com/global/en/products/control/switch/light-touch/small_smd_side",
  wurth: "https://www.we-online.com/en/components/products/em/switches/tact_switches/ws-tasu",
  ck: "https://www.ckswitches.com/media/1482/kms.pdf",
  es3340: "https://www.e-switch.com/product/tl3340-series-subminiature-smt-right-angle-tactile-switch/",
  es3330: "https://www.e-switch.com/product/tl3330-series-smt-right-angle-tactile-switch/",
  es3360: "https://www.e-switch.com/product/tl3360-series-smt-right-angle-tactile-switch/",
  es3336: "https://www.e-switch.com/product/tl3336-series-sealed-smt-right-angle-tactile-switch/",
};

const images = {
  benchmarkDrawing: "C:/Users/gummy/AppData/Local/Temp/codex-clipboard-7ac0fb5f-34a4-41ee-89ed-fb3c847e9291.png",
  benchmarkPhoto: "C:/Users/gummy/AppData/Local/Temp/codex-clipboard-de71bdff-198f-4772-8009-0ca66bdcf4e2.png",
  sksj: "https://tech.alpsalpine.com/cms.media/product_detail_main_sksjlee010_917bc97b62.jpg",
  sksc: "https://tech.alpsalpine.com/cms.media/product_detail_main_sksclce010_b8d84d7863.jpg",
  sktd: "https://tech.alpsalpine.com/cms.media/product_detail_main_sktdlde010_950720f6cc.jpg",
  panaP7: "https://industry.panasonic.com/ac/e/control/switch/light-touch/thumbnail/3457_04.png",
  panaEvpat: "https://industry.panasonic.com/ac/e/control/switch/light-touch/thumbnail/3461_02.png",
  panaEvqp4: "https://industry.panasonic.com/ac/e/control/switch/light-touch/thumbnail/3463_01.png",
  wurthMid: "https://www.we-online.com/components/media/o158918v209%20WS-TASU-434381035816.jpg",
};

const buy = (part) => `https://www.digikey.jp/ja/products?keywords=${encodeURIComponent(part)}`;
const row = (maker, series, part, status, mount, l, w, h, force, travel, life, protection, variant, source, note = "", confidence = "メーカー公式") => ({
  maker, series, part, status, mount, l, w, h, force, travel, life, protection, variant,
  source, purchase: buy(part), note, confidence,
});

const candidates = [
  row("Alps Alpine","SKSJ", "SKSJLCE010","標準品","横押しSMD／ハーフマウント",4.8,3.25,3.3,0.8,0.25,2000000,"なし","0.8N・長寿命",urls.alpsSksj,"今回の最軽量。軸位置を3.3mm級に合わせやすい。"),
  row("Alps Alpine","SKSJ", "SKSJLEE010","標準品","横押しSMD／ハーフマウント",4.8,3.25,3.3,1.6,0.4,1000000,"なし","1.6N・長ストローク",urls.alpsSksj,"軽さと寿命のバランスが良い。"),
  row("Alps Alpine","SKSC", "SKSCLCE010","標準品","横押しSMD／ハーフマウント",3.5,3.55,1.25,1.6,0.2,100000,"なし","接地端子なし・ボスなし",urls.alpsSksc,"極薄・小面積。アクチュエータ中心高さの機構調整が必要。"),
  row("Alps Alpine","SKSC", "SKSCLDE010","標準品","横押しSMD／ハーフマウント",3.5,3.55,1.25,1.6,0.2,100000,"なし","接地端子なし・ボスあり",urls.alpsSksc),
  row("Alps Alpine","SKSC", "SKSCPCE010","標準品","横押しSMD／ハーフマウント",3.5,3.55,1.25,1.6,0.2,100000,"なし","接地端子あり・ボスなし",urls.alpsSksc),
  row("Alps Alpine","SKSC", "SKSCPDE010","標準品","横押しSMD／ハーフマウント",3.5,3.55,1.25,1.6,0.2,100000,"なし","接地端子あり・ボスあり",urls.alpsSksc),
  row("Alps Alpine","SKSC", "SKSCLAE010","標準品","横押しSMD／ハーフマウント",3.5,3.55,1.25,2.2,0.2,100000,"なし","接地端子なし・ボスなし",urls.alpsSksc),
  row("Alps Alpine","SKSC", "SKSCLBE010","標準品","横押しSMD／ハーフマウント",3.5,3.55,1.25,2.2,0.2,100000,"なし","接地端子なし・ボスあり",urls.alpsSksc),
  row("Alps Alpine","SKSC", "SKSCPAE010","標準品","横押しSMD／ハーフマウント",3.5,3.55,1.25,2.2,0.2,100000,"なし","接地端子あり・ボスなし",urls.alpsSksc),
  row("Alps Alpine","SKSC", "SKSCPBE010","標準品","横押しSMD／ハーフマウント",3.5,3.55,1.25,2.2,0.2,100000,"なし","接地端子あり・ボスあり",urls.alpsSksc),
  row("Alps Alpine","SKTD", "SKTDLDE010","標準品","横押しSMD",3.9,2.9,1.55,1.6,0.15,200000,"IP6X相当/IPX7","防水防塵",urls.alpsSktd,"小型かつ防水防塵。"),
  row("Alps Alpine","SKTD", "SKTDLGE010","標準品","横押しSMD",3.9,2.9,1.55,2.0,0.15,200000,"IP6X相当/IPX7","防水防塵・2.0N",urls.alpsSktd),
  row("Alps Alpine","SKSL", "SKSLLAE010","標準品","横押しSMD／ハーフマウント",4.5,2.6,2.2,1.6,0.15,600000,"なし","1.6N",urls.alpsSksl,"細長く基板端に置きやすい。"),
  row("Alps Alpine","SKSL", "SKSLLBE010","標準品","横押しSMD／ハーフマウント",4.5,2.6,2.2,2.2,0.15,600000,"なし","2.2N",urls.alpsSksl),
  row("Alps Alpine","SKSL", "SKSLLCE010","標準品","横押しSMD／ハーフマウント",4.5,2.6,2.2,3.5,0.15,100000,"なし","3.5N・車載対応",urls.alpsSksl),
  row("Alps Alpine","SKRT", "SKRTLAE010","標準品","横押しSMD",4.5,3.4,3.3,1.6,0.2,100000,"なし","ガイドボスあり",urls.alpsSkrt),
  row("Alps Alpine","SKRT", "SKRTLBE010","標準品","横押しSMD",4.5,3.4,3.3,1.6,0.2,100000,"なし","ガイドボスなし",urls.alpsSkrt),
  row("Alps Alpine","SKSN", "SKSNLME010","標準品","横押しSMD／ミッドマウント",7.5,3.0,3.5,1.6,0.15,1000000,"なし","長寿命",urls.alpsSksn),
  row("Alps Alpine","SKSN", "SKSNLAE010","標準品","横押しSMD／ミッドマウント",6.2,3.0,3.5,2.4,0.2,500000,"なし","2.4N",urls.alpsSksn),
  row("Alps Alpine","SKSN", "SKSNLKE010","標準品","横押しSMD／ミッドマウント",6.2,3.0,3.5,4.5,0.2,100000,"なし","4.5N・車載",urls.alpsSksn),
  row("Alps Alpine","SKSN", "SKSNLHE010","標準品","横押しSMD／ミッドマウント",6.2,3.0,3.5,5.0,0.2,100000,"なし","5.0N・車載",urls.alpsSksn),
  row("Alps Alpine","SKSN", "SKSNLPE010","標準品","横押しSMD／ミッドマウント",7.5,4.07,3.5,1.6,0.23,1000000,"なし","新型・長寿命",urls.alpsSksn),

  row("Panasonic Industry","EVQP7", "EVQP7K01P","掲載中","横押しSMD／J曲げ",3.5,3.55,1.35,1.6,0.2,100000,"なし","ボスなし", "https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evqp7k01p","添付品よりかなり薄い。"),
  row("Panasonic Industry","EVQP7", "EVQP7L01P","掲載中","横押しSMD／ストレート",3.5,3.55,1.35,1.6,0.2,100000,"なし","ボスあり", "https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evqp7l01p"),
  row("Panasonic Industry","EVQP7", "EVQP7A01P","掲載中","横押しSMD／ストレート",3.5,3.55,1.35,2.2,0.2,100000,"なし","ボスなし", "https://industry.panasonic.com/ap/en/products/control/switch/light-touch/number/evqp7a01p"),
  row("Panasonic Industry","EVQP7", "EVQP7B01P","掲載中","横押しSMD／J曲げ",3.5,3.55,1.35,2.2,0.2,100000,"なし","ボスなし", "https://industry.panasonic.com/ap/en/products/control/switch/light-touch/number/evqp7b01p"),
  row("Panasonic Industry","EVQPU", "EVQPUJ02K","掲載中","横押しSMD／ストレート",4.7,4.5,1.65,1.6,0.3,100000,"なし","ボスなし", "https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evqpuj02k"),
  row("Panasonic Industry","EVQPU", "EVQPUK02K","掲載中","横押しSMD／J曲げ",4.7,4.5,1.65,1.6,0.3,100000,"なし","ボスなし", "https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evqpuk02k"),
  row("Panasonic Industry","EVQPU", "EVQPUC02K","掲載中","横押しSMD／ストレート",4.7,4.5,1.65,2.2,0.3,100000,"なし","ボスあり", "https://industry.panasonic.com/ap/en/products/control/switch/light-touch/number/evqpuc02k"),

  row("Würth Elektronik","WS-TASU", "434331013822","Active","横押しSMD",3.5,2.9,1.35,2.157,0.2,100000,"なし","220gf",urls.wurth),
  row("Würth Elektronik","WS-TASU", "434351045816","Active","横押しSMD",4.7,3.5,1.65,1.569,0.2,200000,"なし","160gf・白",urls.wurth),
  row("Würth Elektronik","WS-TASU", "434331045822","Active","横押しSMD",4.7,3.5,1.65,2.157,0.2,100000,"なし","220gf・黒",urls.wurth),
  row("Würth Elektronik","WS-TASU", "434353045816","Active","横押しSMD／J曲げ",4.7,3.5,1.65,1.569,0.2,200000,"なし","160gf・白",urls.wurth),
  row("Würth Elektronik","WS-TASU", "434333045822","Active","横押しSMD／J曲げ",4.7,3.5,1.65,2.157,0.2,100000,"なし","220gf・黒",urls.wurth),
  row("Würth Elektronik","WS-TASU", "436351045816","Active","横押しSMD",4.7,3.5,1.65,1.569,0.2,200000,"なし","ボスあり・160gf",urls.wurth),
  row("Würth Elektronik","WS-TASU", "436331045822","Active","横押しSMD",4.7,3.5,1.65,2.157,0.2,100000,"なし","ボスあり・220gf",urls.wurth),
  row("Würth Elektronik","WS-TASU", "436353045816","Active","横押しSMD／J曲げ",4.7,3.5,1.65,1.569,0.2,200000,"なし","ボスあり・160gf",urls.wurth),
  row("Würth Elektronik","WS-TASU", "436333045822","Active","横押しSMD／J曲げ",4.7,3.5,1.65,2.157,0.2,100000,"なし","ボスあり・220gf",urls.wurth),

  row("C&K / Littelfuse","KMS", "KMS221G LFS","注文可","横押しSMT／ガルウィング",4.2,3.55,1.42,2.0,0.25,100000,"IP40","銀接点・ボスなし",urls.ck),
  row("C&K / Littelfuse","KMS", "KMS221GP LFS","注文可","横押しSMT／ガルウィング",4.2,3.55,1.42,2.0,0.25,100000,"IP40","銀接点・ボスあり",urls.ck),
  row("C&K / Littelfuse","KMS", "KMS223G LFG","注文可","横押しSMT／ガルウィング",4.2,3.55,1.42,2.0,0.25,100000,"IP40","金接点・ボスなし",urls.ck),
  row("C&K / Littelfuse","KMS", "KMS223GP LFG","注文可","横押しSMT／ガルウィング",4.2,3.55,1.42,2.0,0.25,100000,"IP40","金接点・ボスあり",urls.ck),
  row("C&K / Littelfuse","KMS", "KMS231G LFS","注文可","横押しSMT／ガルウィング",4.2,3.55,1.42,2.9,0.3,100000,"IP40","銀接点・ボスなし",urls.ck),
  row("C&K / Littelfuse","KMS", "KMS231GP LFS","注文可","横押しSMT／ガルウィング",4.2,3.55,1.42,2.9,0.3,100000,"IP40","銀接点・ボスあり",urls.ck),
  row("C&K / Littelfuse","KMS", "KMS233G LFG","注文可","横押しSMT／ガルウィング",4.2,3.55,1.42,2.9,0.3,100000,"IP40","金接点・ボスなし",urls.ck),
  row("C&K / Littelfuse","KMS", "KMS233GP LFG","注文可","横押しSMT／ガルウィング",4.2,3.55,1.42,2.9,0.3,100000,"IP40","金接点・ボスあり",urls.ck),

  row("E-Switch","TL3340", "TL3340AF130QG","構成可能","横押し／ライトアングルSMT",4.25,3.3,3.4,1.275,0.2,100000,"なし","Aアクチュエータ・130gf",urls.es3340,"軽い130gf仕様。個別在庫は購入先で確認。","メーカー公式（シリーズ構成）"),
  row("E-Switch","TL3340", "TL3340AF160QG","在庫掲載","横押し／ライトアングルSMT",4.25,3.3,3.4,1.569,0.2,100000,"なし","Aアクチュエータ・160gf",urls.es3340),
  row("E-Switch","TL3340", "TL3340BF160QG","共通構成","横押し／ライトアングルSMT",4.25,3.3,null,1.569,0.2,100000,"なし","Bアクチュエータ・160gf",urls.es3340,"高さは個別図面で確認。"),
  row("E-Switch","TL3340", "TL3340CF160QG","共通構成","横押し／ライトアングルSMT",4.25,3.3,null,1.569,0.2,100000,"なし","Cアクチュエータ・160gf",urls.es3340,"高さは個別図面で確認。"),
  row("E-Switch","TL3340", "TL3340AF260QG","共通構成","横押し／ライトアングルSMT",4.25,3.3,3.4,2.55,0.2,100000,"なし","Aアクチュエータ・260gf",urls.es3340),
  row("E-Switch","TL3330", "TL3330AF130QG","共通構成","横押し／ライトアングルSMT",7.8,2.5,3.5,1.275,0.25,50000,"なし","130gf",urls.es3330),
  row("E-Switch","TL3330", "TL3330AF260QG","共通構成","横押し／ライトアングルSMT",7.8,2.5,3.5,2.55,0.25,30000,"なし","260gf",urls.es3330),
  row("E-Switch","TL3360", "TL3360AF185Q","在庫掲載","横押し／ライトアングルSMT",6.5,6.5,2.3,1.814,0.15,200000,"なし","Aアクチュエータ・185gf",urls.es3360,"サイズは大きめの比較用。"),
  row("E-Switch","TL3360", "TL3360AF260Q","共通構成","横押し／ライトアングルSMT",6.5,6.5,2.3,2.55,0.15,200000,"なし","Aアクチュエータ・260gf",urls.es3360),
  row("E-Switch","TL3360", "TL3360BF185Q","共通構成","横押し／ライトアングルSMT",6.5,6.5,null,1.814,0.15,200000,"なし","Bアクチュエータ・185gf",urls.es3360),
  row("E-Switch","TL3360", "TL3360CF185Q","共通構成","横押し／ライトアングルSMT",6.5,6.5,4.0,1.814,0.15,200000,"なし","Cアクチュエータ・185gf",urls.es3360),
  row("E-Switch","TL3360", "TL3360DF185Q","共通構成","横押し／ライトアングルSMT",6.5,6.5,null,1.814,0.15,200000,"なし","Dアクチュエータ・185gf",urls.es3360),
  row("E-Switch","TL3336", "TL3336AF160Q","在庫掲載","横押し／ライトアングルSMT",7.0,7.1,null,1.569,0.35,100000,"IP67","160gf",urls.es3336,"防水だが大きめ。"),
  row("E-Switch","TL3336", "TL3336AF260Q","共通構成","横押し／ライトアングルSMT",7.0,7.1,null,2.55,0.35,100000,"IP67","260gf",urls.es3336),
  row("E-Switch","TL3336", "TL3336AF320Q","共通構成","横押し／ライトアングルSMT",7.0,7.1,null,3.138,0.35,100000,"IP67","320gf",urls.es3336),

  { maker:"AliExpress汎用品", series:"TS-1246VW", part:"TS-1246VW", status:"通販掲載", mount:"横押しSMD／基板端", l:6.2, w:3.5, h:3.5, force:null, travel:null, life:null, protection:"不明", variant:"添付写真の基準品", source:"https://ja.aliexpress.com/item/1005007303304017.html", purchase:"https://ja.aliexpress.com/item/1005007303304017.html", note:"画像表記 3.5×6.2×3.5。押下圧は販売ページ／実測で要確認。", confidence:"ユーザー添付画像" },
  { maker:"AliExpress汎用品", series:"SW101系", part:"商品ID 1005012595826990", status:"通販掲載", mount:"横押しSMD／基板端", l:4.75, w:3.5, h:3.05, force:null, travel:null, life:null, protection:"不明", variant:"添付寸法図の基準品", source:"https://ja.aliexpress.com/item/1005012595826990.html", purchase:"https://ja.aliexpress.com/item/1005012595826990.html", note:"寸法図から読める概略値。発注前に選択肢ごとの図面確認が必要。", confidence:"ユーザー添付寸法図（概略）" },
];

candidates.push(
  row("Panasonic Industry","EVPAT","EVPAT1L1B000","掲載中","横押しSMD／エッジマウント",3.4,1.9,1.65,1.0,0.08,500000,"IP67","Insert端子・接地端子あり","https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evpat1l1b000","超小型・1.0N。今回の構造条件に最も合う有力候補。"),
  row("Panasonic Industry","EVPAT","EVPAT2L1B000","掲載中","横押しSMD／エッジマウント",3.4,1.9,1.65,1.6,0.11,500000,"IP67","Insert端子・接地端子あり","https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evpat2l1b000","超小型・IP67。"),
  row("Panasonic Industry","EVPAV","EVPAVAA1A","掲載中","横押しSMD／エッジマウント",2.8,2.65,1.95,1.6,0.13,300000,"なし","Flat端子","https://industry.panasonic.com/ap/en/products/control/switch/light-touch/number/evpavaa1a","最小クラス。基板切欠きと実装公差を要確認。"),
  row("Panasonic Industry","EVQP4","EVQP40B3M","掲載中","横押しSMD／エッジマウント",6.2,3.0,3.5,1.0,0.25,1000000,"なし","Flat端子・接地端子あり","https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evqp40b3m","TS-1246VWとほぼ同じ外形級で1.0N。非常に有力。"),
  row("Panasonic Industry","EVQP4","EVQP42B3M","掲載中","横押しSMD／エッジマウント",6.2,3.0,3.5,1.6,0.25,1000000,"なし","Flat端子・接地端子あり","https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evqp42b3m","TS-1246VWとほぼ同じ外形級。"),
  row("Panasonic Industry","EVQP4","EVQP4KB3Q","掲載中","横押しSMD／エッジマウント",6.2,3.4,3.5,3.5,0.7,500000,"なし","車載・Flat端子","https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evqp4kb3q","重いが長ストローク。"),
  row("Panasonic Industry","EVQP4","EVQP4MB3K","掲載中","横押しSMD／エッジマウント",6.2,3.4,3.5,5.0,0.7,200000,"なし","車載・Flat端子","https://industry.panasonic.com/ap/en/products/control/switch/light-touch/number/evqp4mb3k","かなり重い比較用。"),
  row("Panasonic Industry","EVPAE","EVPAEGE2A","掲載中","横押しSMD／エッジマウント",4.5,2.6,2.9,1.6,0.13,200000,"なし","L字端子・接地端子あり","https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evpaege2a","薄いエッジマウント。"),
  row("Panasonic Industry","EVPAE","EVPAEBB2A","掲載中","横押しSMD／エッジマウント",4.5,2.6,2.9,1.6,0.15,200000,"なし","Flat端子・接地端子あり","https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evpaebb2a"),
  row("Panasonic Industry","EVPAE","EVPAEDB2A","掲載中","横押しSMD／エッジマウント",4.5,2.6,2.9,3.0,0.15,200000,"なし","Flat端子・接地端子あり・車載","https://industry.panasonic.com/global/en/products/control/switch/light-touch/number/evpaedb2a"),
  row("Würth Elektronik","WS-TASU","434381035816","Active","横押しSMD／ミッドマウント",6.2,3.0,3.5,1.569,0.2,1000000,"なし","160gf・青","https://www.we-online.com/en/components/products/WS_TASU_SMT_TACT_SWITCH_6_2_X_3_0","TS-1246VWと同じ外形級。公式にMid-Mount明記。")
);

for (const x of candidates) {
  if (["EVPAT","EVPAV","EVQP4","EVPAE"].includes(x.series)) x.fit = "高（エッジマウント）";
  else if (x.series === "SKSN" || x.part === "434381035816") x.fit = "高（ミッドマウント）";
  else if (["SKSJ","SKSC","SKSL"].includes(x.series)) x.fit = "高（ハーフマウント）";
  else if (x.maker === "AliExpress汎用品") x.fit = "高（画像判断・要図面確認）";
  else if (["EVQP7","EVQPU"].includes(x.series)) x.fit = "中（取付方向を図面確認）";
  else x.fit = "低（通常ライトアングル）";
}

const scoreForSort = (x) => x.force == null ? -999 : 100 - (x.l*x.w)*1.8 - x.force*16 - Math.max(0,(x.h ?? 3.5)-3.5)*4;
candidates.sort((a,b) => scoreForSort(b)-scoreForSort(a));

const topParts = ["EVPAT1L1B000","EVQP40B3M","434381035816","EVPAVAA1A","EVPAT2L1B000","SKSNLME010","EVPAEGE2A","SKSJLCE010","SKSLLAE010","SKSCLCE010"];
const top = topParts.map(p => candidates.find(x => x.part === p)).filter(Boolean);

const wb = Workbook.create();
const summary = wb.worksheets.add("おすすめ");
const sheet = wb.worksheets.add("候補一覧");
const criteria = wb.worksheets.add("評価基準");
summary.showGridLines = false;
sheet.showGridLines = false;
criteria.showGridLines = false;

criteria.getRange("A1:B8").values = [
  ["評価パラメータ","値"],
  ["面積ペナルティ係数",1.8],
  ["押下圧ペナルティ係数",16],
  ["高さ超過ペナルティ係数",4],
  ["高さ基準 (mm)",3.5],
  ["非常に軽い上限 (N)",1.0],
  ["軽い上限 (N)",1.6],
  ["中程度上限 (N)",2.2],
];
criteria.getRange("A10:B13").values = [
  ["スコア式","100 − 面積×係数 − 押下圧×係数 − 高さ超過×係数"],
  ["注意1","スコアは比較用。軸高さ、押し子形状、ランド強度、在庫を別途確認。"],
  ["注意2","gfからNへの換算は 1 gf = 0.00980665 N。"],
  ["調査日","2026-07-18"],
];
criteria.getRange("A1:B1").format = {fill:"#123B4A",font:{bold:true,color:"#FFFFFF"}};
criteria.getRange("A1:B13").format.borders = {preset:"inside",style:"thin",color:"#D6E2E8"};
criteria.getRange("A:A").format.columnWidth = 28;
criteria.getRange("B:B").format.columnWidth = 70;
criteria.getRange("B2:B8").format.numberFormat = "0.00";

const headers = ["メーカー","シリーズ","型番","供給状態","実装／機構","L (mm)","W (mm)","H (mm)","実装面積 (mm²)","押下圧 (N)","押下圧 (gf)","ストローク (mm)","寿命 (回)","保護","端子／ボス等","サイズ区分","押下圧評価","比較スコア","メーカー仕様URL","購入・検索URL","選定メモ","根拠レベル","ミッド／エッジ適合"];
sheet.getRange("A1:W1").values = [headers];
const displayPart = (p) => /^\d+$/.test(p) ? `P/N ${p}` : p;
const values = candidates.map(x => [x.maker,x.series,displayPart(x.part),x.status,x.mount,x.l,x.w,x.h,"",x.force,"",x.travel,x.life,x.protection,x.variant,"","","",x.source,x.purchase,x.note,x.confidence,x.fit]);
sheet.getRange(`A2:W${candidates.length+1}`).values = values;
sheet.getRange(`C2:C${candidates.length+1}`).format.numberFormat = "@";
const last = candidates.length + 1;
sheet.getRange("I2").formulas = [["=IF(OR(F2=\"\",G2=\"\"),\"\",F2*G2)"]];
sheet.getRange(`I2:I${last}`).fillDown();
sheet.getRange("K2").formulas = [["=IF(J2=\"\",\"\",J2*101.971621)"]];
sheet.getRange(`K2:K${last}`).fillDown();
sheet.getRange("P2").formulas = [["=IF(J2=\"\",\"要確認\",IF(AND(MAX(F2,G2)<=5,J2<='評価基準'!$B$7),\"小型・軽め\",IF(MAX(F2,G2)<=5,\"小型\",IF(MAX(F2,G2)<=6.5,\"標準\",\"大きめ参考\"))))"]];
sheet.getRange(`P2:P${last}`).fillDown();
sheet.getRange("Q2").formulas = [["=IF(J2=\"\",\"不明\",IF(J2<='評価基準'!$B$6,\"非常に軽い\",IF(J2<='評価基準'!$B$7,\"軽い\",IF(J2<='評価基準'!$B$8,\"中程度\",\"重め\"))))"]];
sheet.getRange(`Q2:Q${last}`).fillDown();
sheet.getRange("R2").formulas = [["=IF(OR(I2=\"\",J2=\"\"),\"\",MAX(0,ROUND(100-I2*'評価基準'!$B$2-J2*'評価基準'!$B$3-MAX(0,H2-'評価基準'!$B$5)*'評価基準'!$B$4,1)))"]];
sheet.getRange(`R2:R${last}`).fillDown();

const table = sheet.tables.add(`A1:W${last}`, true, "SidePushCandidates");
table.style = "TableStyleMedium2";
table.showBandedRows = true;
table.showFilterButton = true;
sheet.freezePanes.freezeRows(1);
sheet.freezePanes.freezeColumns(3);
sheet.getRange("A1:W1").format = {fill:"#123B4A",font:{bold:true,color:"#FFFFFF",size:10},wrapText:true,verticalAlignment:"center"};
sheet.getRange("A1:V1").format.rowHeight = 32;
sheet.getRange(`A2:W${last}`).format.font = {size:9,color:"#1C2B33"};
sheet.getRange(`F2:M${last}`).format.horizontalAlignment = "right";
sheet.getRange(`F2:L${last}`).format.numberFormat = "0.00";
sheet.getRange(`M2:M${last}`).format.numberFormat = "#,##0";
sheet.getRange(`R2:R${last}`).format.numberFormat = "0.0";
sheet.getRange(`A2:W${last}`).format.verticalAlignment = "top";
sheet.getRange(`U2:U${last}`).format.wrapText = true;
sheet.getRange(`R2:R${last}`).conditionalFormats.add("colorScale", {colors:["#FEE2E2","#FEF3C7","#DCFCE7"],thresholds:["min","50%","max"]});
sheet.getRange(`J2:J${last}`).conditionalFormats.add("colorScale", {colors:["#DCFCE7","#FEF3C7","#FECACA"],thresholds:["min","50%","max"]});
const widths = [20,12,24,12,25,9,9,9,14,12,12,14,14,13,25,15,15,12,55,48,52,23,26];
widths.forEach((w,i)=> sheet.getRangeByIndexes(0,i,last,1).format.columnWidth = w);

summary.getRange("A1:J2").merge();
summary.getRange("A1").values = [["横押しSMDモーメンタリスイッチ 調査結果"]];
summary.getRange("A1:J2").format = {fill:"#123B4A",font:{bold:true,color:"#FFFFFF",size:20},verticalAlignment:"center"};
summary.getRange("A3:J3").merge();
summary.getRange("A3").values = [["ホイール軸延長上のサイドプッシュ用途｜小型・軽い順を優先｜2026-07-18"]];
summary.getRange("A3:J3").format = {fill:"#E8F2F5",font:{color:"#315B67",italic:true,size:10}};

summary.getRange("A5:B7").merge(); summary.getRange("A5").formulas = [[`=COUNTA('候補一覧'!$C$2:$C$${last})`]];
summary.getRange("C5:D7").merge(); summary.getRange("C5").formulas = [[`=COUNTIF('候補一覧'!$Q$2:$Q$${last},\"非常に軽い\")+COUNTIF('候補一覧'!$Q$2:$Q$${last},\"軽い\")`]];
summary.getRange("E5:F7").merge(); summary.getRange("E5").values = [["0.8 N"]];
summary.getRange("G5:J7").merge(); summary.getRange("G5").values = [["最有力: EVPAT1L1B000\nEdge Mount / 3.4×1.9×1.65 mm / 1.0 N"]];
for (const r of ["A5:B7","C5:D7","E5:F7","G5:J7"]) summary.getRange(r).format = {fill:"#F3F8FA",font:{bold:true,color:"#123B4A",size:r==="G5:J7"?12:18},borders:{preset:"outside",style:"thin",color:"#B7CDD5"},horizontalAlignment:"center",verticalAlignment:"center",wrapText:true};
summary.getRange("A8:B8").merge(); summary.getRange("A8").values = [["候補総数"]];
summary.getRange("C8:D8").merge(); summary.getRange("C8").values = [["軽い候補 ≤1.6N"]];
summary.getRange("E8:F8").merge(); summary.getRange("E8").values = [["最小押下圧"]];
summary.getRange("G8:J8").merge(); summary.getRange("G8").values = [["一次試作推奨"]];
summary.getRange("A8:J8").format = {fill:"#D7E8ED",font:{bold:true,color:"#315B67",size:9},horizontalAlignment:"center"};

summary.getRange("A10:J10").merge(); summary.getRange("A10").values = [["まず試す候補（メーカー公式仕様が確認できるもの）"]];
summary.getRange("A10:J10").format = {fill:"#D97706",font:{bold:true,color:"#FFFFFF",size:12}};
const topHeaders = ["優先","メーカー","型番","寸法 L×W×H (mm)","押下圧","ストローク","寿命","特徴","仕様URL","購入検索"];
summary.getRange("A11:J11").values = [topHeaders];
summary.getRange("A11:J11").format = {fill:"#FEF3C7",font:{bold:true,color:"#7C2D12",size:9},wrapText:true};
const topValues = top.map((x,i)=>[i+1,x.maker,displayPart(x.part),`${x.l}×${x.w}×${x.h ?? "要確認"}`,`${x.force} N`,x.travel,x.life,x.protection==="なし"?x.variant:`${x.variant} / ${x.protection}`,x.source,x.purchase]);
summary.getRange(`A12:J${11+top.length}`).values = topValues;
summary.getRange(`C12:C${11+top.length}`).format.numberFormat = "@";
summary.getRange(`A12:J${11+top.length}`).format = {font:{size:9,color:"#1C2B33"},verticalAlignment:"top",wrapText:true,borders:{insideHorizontal:{style:"thin",color:"#D7E4E8"}}};
summary.getRange(`F12:F${11+top.length}`).format.numberFormat = "0.00";
summary.getRange(`G12:G${11+top.length}`).format.numberFormat = "#,##0";

const noteRow = 13 + top.length;
summary.getRange(`A${noteRow}:J${noteRow}`).merge(); summary.getRange(`A${noteRow}`).values = [["設計上の要点: 押下圧だけでなく、ホイール軸中心とアクチュエータ中心の高さ、許容オーバートラベル、横荷重、ランド剥離対策（ボス／補強板）を確認してください。0.8N品は軽い一方、誤操作や振動入力への余裕が小さくなります。"]];
summary.getRange(`A${noteRow}:J${noteRow}`).format = {fill:"#FFF7ED",font:{color:"#7C2D12",size:10},wrapText:true,borders:{preset:"outside",style:"thin",color:"#FDBA74"}};
summary.getRange(`A${noteRow}:J${noteRow}`).format.rowHeight = 48;

summary.getRange(`A${noteRow+2}:E${noteRow+2}`).merge(); summary.getRange(`A${noteRow+2}`).values = [["ユーザー提示の基準品・寸法図"]];
summary.getRange(`F${noteRow+2}:J${noteRow+2}`).merge(); summary.getRange(`F${noteRow+2}`).values = [["ユーザー提示の TS-1246VW 写真"]];
summary.getRange(`A${noteRow+2}:J${noteRow+2}`).format = {fill:"#D7E8ED",font:{bold:true,color:"#315B67"},horizontalAlignment:"center"};

try {
  const [drawing, photo] = await Promise.all([fs.readFile(images.benchmarkDrawing), fs.readFile(images.benchmarkPhoto)]);
  summary.images.add({dataUrl:`data:image/png;base64,${drawing.toString("base64")}`,anchor:{from:{row:noteRow+2,col:0},extent:{widthPx:350,heightPx:260}}});
  summary.images.add({dataUrl:`data:image/png;base64,${photo.toString("base64")}`,anchor:{from:{row:noteRow+2,col:5},extent:{widthPx:350,heightPx:260}}});
} catch (e) { console.log("image embed warning", String(e)); }

const sumWidths = [8,20,24,22,12,12,14,30,52,48];
sumWidths.forEach((w,i)=> summary.getRangeByIndexes(0,i,noteRow+18,1).format.columnWidth = w);
summary.freezePanes.freezeRows(3);
summary.getRange(`A1:J${noteRow+16}`).format.verticalAlignment = "center";

const summaryInspect = await wb.inspect({kind:"table",range:`おすすめ!A1:J${noteRow}`,include:"values,formulas",tableMaxRows:40,tableMaxCols:10,maxChars:8000});
console.log(summaryInspect.ndjson);
const errorScan = await wb.inspect({kind:"match",searchTerm:"#REF!|#DIV/0!|#VALUE!|#NAME\\?|#N/A",options:{useRegex:true,maxResults:100},summary:"formula error scan",maxChars:3000});
console.log(errorScan.ndjson);
const drawingScan = await wb.inspect({kind:"drawing",sheetId:"おすすめ",maxChars:3000});
console.log(drawingScan.ndjson);

const preview1 = await wb.render({sheetName:"おすすめ",range:`A1:J${noteRow+15}`,scale:1.25,format:"png"});
await fs.writeFile(path.join(outDir,"qa_summary.png"),new Uint8Array(await preview1.arrayBuffer()));
const preview2 = await wb.render({sheetName:"候補一覧",range:"A1:W22",scale:1.0,format:"png"});
await fs.writeFile(path.join(outDir,"qa_candidates.png"),new Uint8Array(await preview2.arrayBuffer()));

const xlsx = await SpreadsheetFile.exportXlsx(wb);
await xlsx.save(path.join(outDir,"side_push_smd_switches_20260718.xlsx"));

const benchDrawing = (await fs.readFile(images.benchmarkDrawing)).toString("base64");
const benchPhoto = (await fs.readFile(images.benchmarkPhoto)).toString("base64");
const htmlRows = candidates.map((x,i)=>({...x,idx:i+1,area:x.l&&x.w?+(x.l*x.w).toFixed(2):null,gf:x.force==null?null:+(x.force*101.971621).toFixed(0),score:x.force==null?null:+scoreForSort(x).toFixed(1),image:x.series==="SKSJ"?images.sksj:x.series==="SKSC"?images.sksc:x.series==="SKTD"?images.sktd:x.series==="EVQP7"?images.panaP7:x.series==="EVPAT"?images.panaEvpat:x.series==="EVQP4"?images.panaEvqp4:x.part==="434381035816"?images.wurthMid:""}));
const dataJson = JSON.stringify(htmlRows).replaceAll("<","\\u003c");
const html = `<!doctype html><html lang="ja"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>横押しSMDスイッチ比較</title><style>
:root{--ink:#18313a;--muted:#647982;--teal:#123b4a;--pale:#edf5f7;--amber:#d97706;--line:#d5e2e6;--green:#15803d}*{box-sizing:border-box}body{margin:0;font-family:Inter,"Yu Gothic UI",Meiryo,sans-serif;color:var(--ink);background:#f5f8f9}header{background:linear-gradient(135deg,#102f3a,#1f5b69);color:#fff;padding:36px max(24px,5vw)}header h1{margin:0 0 8px;font-size:clamp(25px,4vw,42px)}header p{margin:0;color:#d4e8ed}.wrap{max-width:1500px;margin:auto;padding:24px}.cards{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px;margin-top:-42px}.card{background:#fff;border:1px solid var(--line);border-radius:14px;padding:18px;box-shadow:0 8px 22px #123b4a18}.card b{display:block;font-size:28px;color:var(--teal)}.card span{color:var(--muted);font-size:13px}.section{background:#fff;border:1px solid var(--line);border-radius:16px;margin:20px 0;padding:20px}.section h2{margin:0 0 14px}.bench{display:grid;grid-template-columns:1fr 1fr;gap:18px}.bench figure{margin:0;background:var(--pale);border-radius:12px;padding:12px;text-align:center}.bench img{max-width:100%;height:240px;object-fit:contain}.bench figcaption{font-size:13px;color:var(--muted)}.tops{display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:12px}.top{border:1px solid var(--line);border-radius:12px;padding:12px;background:#fbfdfe}.top img,.ph{width:100%;height:110px;object-fit:contain;background:#eef4f6;border-radius:8px}.ph{display:grid;place-items:center;font-weight:bold;color:#78909a}.top h3{font-size:15px;margin:8px 0 4px}.pill{display:inline-block;padding:3px 8px;border-radius:999px;background:#dcfce7;color:#166534;font-size:12px;font-weight:bold}.toolbar{display:flex;flex-wrap:wrap;gap:10px;margin-bottom:12px}.toolbar input,.toolbar select{padding:10px 12px;border:1px solid #b9cdd4;border-radius:8px;background:#fff}.toolbar input{min-width:300px;flex:1}.tablewrap{overflow:auto;max-height:76vh;border:1px solid var(--line);border-radius:10px}table{border-collapse:separate;border-spacing:0;width:100%;min-width:1800px;font-size:13px}th{position:sticky;top:0;background:var(--teal);color:#fff;text-align:left;padding:10px;cursor:pointer;z-index:2}td{padding:9px 10px;border-bottom:1px solid #e5edef;vertical-align:top}tr:hover td{background:#f0f7f8}.num{text-align:right;font-variant-numeric:tabular-nums}.light{color:var(--green);font-weight:bold}.unknown{color:#b45309}a{color:#0b6f8a}small.note{display:block;color:var(--muted);margin-top:4px}.foot{font-size:13px;color:var(--muted);line-height:1.7}@media(max-width:900px){.cards{grid-template-columns:1fr 1fr}.tops{grid-template-columns:1fr 1fr}.bench{grid-template-columns:1fr}}@media(max-width:520px){.cards,.tops{grid-template-columns:1fr}.toolbar input{min-width:100%}}
</style></head><body><header><h1>横押しSMDモーメンタリスイッチ比較</h1><p>ホイール軸延長上のサイドプッシュ用途｜小型・軽い候補を優先｜2026-07-18</p></header><main class="wrap"><div class="cards"><div class="card"><b id="count"></b><span>型番・構成候補</span></div><div class="card"><b id="lightCount"></b><span>押下圧 1.6 N 以下</span></div><div class="card"><b>0.8 N</b><span>確認できた最小押下圧</span></div><div class="card"><b>EVPAT1L1B000</b><span>構造・小型優先の第一候補</span></div></div>
<section class="section"><h2>基準画像</h2><div class="bench"><figure><img src="data:image/png;base64,${benchDrawing}" alt="ユーザー提示の寸法図"><figcaption>ユーザー提示の寸法図（SW101系）</figcaption></figure><figure><img src="data:image/png;base64,${benchPhoto}" alt="TS-1246VW"><figcaption>ユーザー提示の TS-1246VW（3.5×6.2×3.5表記）</figcaption></figure></div></section>
<section class="section"><h2>まず試す5候補</h2><div class="tops" id="tops"></div></section>
<section class="section"><h2>全候補</h2><div class="toolbar"><input id="q" placeholder="型番・メーカー・メモを検索"><select id="maker"><option value="">全メーカー</option></select><select id="force"><option value="">押下圧すべて</option><option value="very">≤1.0 N</option><option value="light">≤1.6 N</option><option value="mid">≤2.2 N</option><option value="unknown">不明のみ</option></select><select id="size"><option value="">サイズすべて</option><option value="tiny">最大辺≤5 mm</option><option value="ref">最大辺≤6.5 mm</option></select><select id="fit"><option value="">構造適合すべて</option><option value="high">ミッド／エッジ／ハーフのみ</option></select></div><div class="tablewrap"><table><thead><tr>${["#","画像","メーカー","シリーズ","型番","状態","実装／機構","L","W","H","面積","押下圧N","gf","ストローク","寿命","保護","端子／ボス等","比較スコア","仕様","購入検索","メモ","根拠","構造適合"].map((h,i)=>`<th data-col="${i}">${h}</th>`).join("")}</tr></thead><tbody id="tbody"></tbody></table></div><p id="shown" class="foot"></p></section>
<section class="section foot"><b>用語と読み方</b><br>今回の本命構造は mid-mount / half-mount / edge-mount（基板切欠きへ本体を落とし、反対面側から実装してアクチュエータを基板端へ出す構造）です。普通の right-angle / side-actuated SMT は別構造として残し、「構造適合」で区別しました。押下圧は小さいほど軽く、比較スコアは小さい実装面積と軽い押下圧を優先する相対値です。</section></main><script>
const rows=${dataJson};const topParts=${JSON.stringify(topParts.slice(0,5))};let sortKey="score",sortAsc=false;const esc=s=>String(s??"").replace(/[&<>\"]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c]));const fmt=n=>n==null?"—":Number(n).toLocaleString("ja-JP");
document.getElementById("count").textContent=rows.length;document.getElementById("lightCount").textContent=rows.filter(r=>r.force!=null&&r.force<=1.6).length;const makers=[...new Set(rows.map(r=>r.maker))].sort();document.getElementById("maker").innerHTML+=""+makers.map(x=>'<option>'+esc(x)+'</option>').join("");
document.getElementById("tops").innerHTML=topParts.map(p=>rows.find(r=>r.part===p)).filter(Boolean).map(r=>'<article class="top">'+(r.image?'<img src="'+esc(r.image)+'" alt="'+esc(r.part)+'" loading="lazy">':'<div class="ph">'+esc(r.series)+'</div>')+'<h3>'+esc(r.part)+'</h3><span class="pill">'+r.force+' N / '+r.gf+' gf</span><p>'+r.l+'×'+r.w+'×'+(r.h??'?')+' mm</p><a href="'+esc(r.source)+'" target="_blank" rel="noreferrer">メーカー仕様</a></article>').join("");
function filtered(){const q=document.getElementById("q").value.toLowerCase(),m=document.getElementById("maker").value,f=document.getElementById("force").value,s=document.getElementById("size").value,fit=document.getElementById("fit").value;return rows.filter(r=>{const text=Object.values(r).join(" ").toLowerCase();if(q&&!text.includes(q))return false;if(m&&r.maker!==m)return false;if(f==="very"&&!(r.force!=null&&r.force<=1))return false;if(f==="light"&&!(r.force!=null&&r.force<=1.6))return false;if(f==="mid"&&!(r.force!=null&&r.force<=2.2))return false;if(f==="unknown"&&r.force!=null)return false;if(s==="tiny"&&Math.max(r.l,r.w)>5)return false;if(s==="ref"&&Math.max(r.l,r.w)>6.5)return false;if(fit==="high"&&!String(r.fit).startsWith("高"))return false;return true}).sort((a,b)=>{const av=a[sortKey]??-Infinity,bv=b[sortKey]??-Infinity;return (typeof av==="string"?av.localeCompare(bv,"ja"):av-bv)*(sortAsc?1:-1)})}
function render(){const a=filtered();document.getElementById("tbody").innerHTML=a.map(r=>'<tr><td class="num">'+r.idx+'</td><td>'+(r.image?'<img src="'+esc(r.image)+'" alt="" loading="lazy" style="width:70px;height:50px;object-fit:contain">':'—')+'</td><td>'+esc(r.maker)+'</td><td>'+esc(r.series)+'</td><td><b>'+esc(r.part)+'</b></td><td>'+esc(r.status)+'</td><td>'+esc(r.mount)+'</td><td class="num">'+fmt(r.l)+'</td><td class="num">'+fmt(r.w)+'</td><td class="num">'+fmt(r.h)+'</td><td class="num">'+fmt(r.area)+'</td><td class="num '+(r.force==null?'unknown':r.force<=1.6?'light':'')+'">'+fmt(r.force)+'</td><td class="num">'+fmt(r.gf)+'</td><td class="num">'+fmt(r.travel)+'</td><td class="num">'+fmt(r.life)+'</td><td>'+esc(r.protection)+'</td><td>'+esc(r.variant)+'</td><td class="num">'+fmt(r.score)+'</td><td><a href="'+esc(r.source)+'" target="_blank" rel="noreferrer">仕様</a></td><td><a href="'+esc(r.purchase)+'" target="_blank" rel="noreferrer">取扱検索</a></td><td>'+esc(r.note)+'</td><td>'+esc(r.confidence)+'</td><td>'+esc(r.fit)+'</td></tr>').join("");document.getElementById("shown").textContent=a.length+' / '+rows.length+' 件を表示'}
["q","maker","force","size","fit"].forEach(id=>document.getElementById(id).addEventListener(id==="q"?"input":"change",render));document.querySelectorAll("th").forEach((th,i)=>th.addEventListener("click",()=>{const keys=["idx","image","maker","series","part","status","mount","l","w","h","area","force","gf","travel","life","protection","variant","score",null,null,null,null,"fit"];const k=keys[i];if(!k)return;if(sortKey===k)sortAsc=!sortAsc;else{sortKey=k;sortAsc=true}render()}));render();
</script></body></html>`;
await fs.writeFile(path.join(outDir,"side_push_smd_switches_20260718.html"),html,"utf8");
console.log(JSON.stringify({outDir,count:candidates.length,xlsx:"side_push_smd_switches_20260718.xlsx",html:"side_push_smd_switches_20260718.html"}));
