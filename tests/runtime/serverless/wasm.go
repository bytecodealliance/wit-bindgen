package main

import (
	. "wit_serverless_go/gen"
)

func init() {
	a := HttpImpl{}
	SetHandler(a)
}

type HttpImpl struct {
}

func (i HttpImpl) HandleHttp(req HandlerRequest) Result[HandlerResponse, HandlerHttpError] {
	for _, header := range req.Headers {
		println(header.F0)
		println(header.F1)
	}
	for _, arg := range req.Params {
		println(arg.F0)
		println(arg.F1)
	}
	response := HandlerResponse{}
	response.Status = 200
	response.Body = Some[[]uint8]([]byte("hello world!"))

	var res Result[HandlerResponse, HandlerHttpError]
	res.Set(response)
	return res
}

func main() {}
