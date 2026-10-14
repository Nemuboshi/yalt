export namespace main {
	
	export class Settings {
	    version: number;
	    api_base: string;
	    api_method: string;
	    api_key: string;
	    model: string;
	    timeout_seconds: number;
	    user_agent: string;
	    source: string;
	    target: string;
	    stream: boolean;
	    max_tokens: number;
	    temperature: number;
	    instruction: string;
	    glossary: translation.Glossary[];
	    disable_thinking: boolean;
	    tray_enabled: boolean;
	    close_to_tray: boolean;
	    start_hidden: boolean;
	
	    static createFrom(source: any = {}) {
	        return new Settings(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.version = source["version"];
	        this.api_base = source["api_base"];
	        this.api_method = source["api_method"];
	        this.api_key = source["api_key"];
	        this.model = source["model"];
	        this.timeout_seconds = source["timeout_seconds"];
	        this.user_agent = source["user_agent"];
	        this.source = source["source"];
	        this.target = source["target"];
	        this.stream = source["stream"];
	        this.max_tokens = source["max_tokens"];
	        this.temperature = source["temperature"];
	        this.instruction = source["instruction"];
	        this.glossary = this.convertValues(source["glossary"], translation.Glossary);
	        this.disable_thinking = source["disable_thinking"];
	        this.tray_enabled = source["tray_enabled"];
	        this.close_to_tray = source["close_to_tray"];
	        this.start_hidden = source["start_hidden"];
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class TranslationInput {
	    requestID: string;
	    text: string;
	    source: string;
	    target: string;
	
	    static createFrom(source: any = {}) {
	        return new TranslationInput(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.requestID = source["requestID"];
	        this.text = source["text"];
	        this.source = source["source"];
	        this.target = source["target"];
	    }
	}

}

export namespace translation {
	
	export class Detection {
	    code: string;
	    name: string;
	    uncertain: boolean;
	    confidence: number;
	    target: string;
	
	    static createFrom(source: any = {}) {
	        return new Detection(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.code = source["code"];
	        this.name = source["name"];
	        this.uncertain = source["uncertain"];
	        this.confidence = source["confidence"];
	        this.target = source["target"];
	    }
	}
	export class Glossary {
	    source: string;
	    target: string;
	
	    static createFrom(source: any = {}) {
	        return new Glossary(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.source = source["source"];
	        this.target = source["target"];
	    }
	}
	export class Language {
	    code: string;
	    name: string;
	
	    static createFrom(source: any = {}) {
	        return new Language(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.code = source["code"];
	        this.name = source["name"];
	    }
	}

}

