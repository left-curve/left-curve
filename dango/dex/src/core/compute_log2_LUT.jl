using Printf

num_points = 6144

for i in 0:num_points-1
    x = BigFloat(1) + BigFloat(i//num_points)
    log2_x = log2(x)
    @printf("%i,\n", log2_x*10^18)
end