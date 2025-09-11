const puppeteer = require('puppeteer');
const config = require("./config.json");

(async () => {
  const args = process.argv;
  const browser = await puppeteer.launch({
    executablePath: `${config.browser}`,
    headless: true,
    args: ["--incognito", "--no-sandbox", "--disable-setuid-sandbox"]
  });
  const page = await browser.newPage();

  // login
  await page.goto('https://essexnorthshore.schoology.com/');

  await page.type('input[type="email"]', `${args[2]}`); // <-- config value
  await page.click('#identifierNext');
  await page.waitForNavigation({ waitUntil: 'networkidle0' });


  await page.type('input[type="password"]', `${args[3]}`); // <-- config value
  await page.click('#passwordNext');

  await page.waitForNavigation({ waitUntil: 'networkidle0' });

  // Cookie time
  let cookies = await browser.cookies()

  const sessCookie = cookies.find(cookies =>
    cookies.name.startsWith("SESS") &&
    cookies.domain === ".essexnorthshore.schoology.com"
  );

  if (sessCookie) {
    let cookie = `${sessCookie.name}=${sessCookie.value}`;
    console.log(cookie);
  }

  browser.close();
})();

